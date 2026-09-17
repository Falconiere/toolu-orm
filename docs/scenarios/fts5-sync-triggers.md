# FTS5 synchronization triggers

**Feature:** `Fts5Table::sync_content()` / `#[fts5_table(..., sync_content = true)]` records an [`Fts5Sync`] on `TableDef` and `SnapshotTable`; the diff creates, drops and replaces the three triggers SQLite needs to keep an external-content FTS5 index correct, and puts them back whenever either table is recreated.
**Drivers:** libsql and rusqlite (SQLite only). On Postgres both operations emit a skip comment.
**Reading one:** this page owns the write side; [FTS5 queries](fts5-queries.md) searches the index, and [Virtual tables (FTS5)](virtual-tables.md) declares it.

## What is proven

### The problem it solves

An external-content FTS5 table stores no rows of its own: it indexes another
table and SQLite leaves keeping the two in step to the application. `rebuild`
indexes the rows that exist when it runs and nothing afterwards, so every later
insert, update and delete has to be mirrored by a trigger. Left hand-written,
those triggers are invisible to the registry and the snapshot, and they are
destroyed without warning whenever generated DDL recreates either table — the
FTS table for a tokenizer change, or the **content** table for a column change
SQLite can only apply by rebuilding it.

### The declaration

`sync_content()` is opt-in and deliberately covers only the safe 1:1 case: the
content table is what `content = '…'` names, the rowid is what
`content_rowid = '…'` names, and every FTS column reads the same-named content
column. Anything else stays hand-written. It adds no module argument — the
declaration lives beside `kind`, in `TableDef.fts5_sync`:

```json
"fts5_sync": {
  "content_table": "memories",
  "content_rowid": "id",
  "columns": ["body", "note"],
  "indexed_columns": ["body"]
}
```

A table that did not opt in writes no `fts5_sync` key at all, so a snapshot
written before this existed is unchanged and loads as unsynchronized.

### The generated SQL

Trigger names key on the FTS table alone — `toolu_fts5_<fts>_insert`,
`_delete`, `_update` — so two indexes over one content table never collide and
a drop needs nothing but the index's name.

```sql
CREATE TRIGGER "toolu_fts5_memory_fts_insert" AFTER INSERT ON "memories" BEGIN
  INSERT INTO "memory_fts" ("rowid", "body", "note")
    VALUES (new."id", new."body", new."note");
END;
CREATE TRIGGER "toolu_fts5_memory_fts_delete" AFTER DELETE ON "memories" BEGIN
  INSERT INTO "memory_fts" ("memory_fts", "rowid", "body", "note")
    VALUES ('delete', old."id", old."body", old."note");
END;
CREATE TRIGGER "toolu_fts5_memory_fts_update" AFTER UPDATE OF "id", "body" ON "memories" BEGIN
  INSERT INTO "memory_fts" ("memory_fts", "rowid", "body", "note")
    VALUES ('delete', old."id", old."body", old."note");
  INSERT INTO "memory_fts" ("rowid", "body", "note")
    VALUES (new."id", new."body", new."note");
END;
INSERT INTO "memory_fts"("memory_fts") VALUES('rebuild');
```

Three things about that shape:

* the delete uses FTS5's special `'delete'` command, which reverses the index
  entries from the values the row was indexed under, so every one is read from
  `old`;
* the update trigger is armed with `AFTER UPDATE OF <rowid>, <indexed columns>`
  and nothing else, so a write that only touches `UNINDEXED` data costs
  nothing. The trigger body still carries every column, because the `'delete'`
  command takes the whole row;
* the `rebuild` comes **after** the triggers, so creating the synchronization
  always ends with an index that matches the content table. For that reason a
  synchronized table's `RecreateFts5FromContent` leaves its own `rebuild` out —
  otherwise the index would be built twice, and once too early.

### The lifecycle

`DropFts5SyncTriggers` sits in the diff's drop tier and
`CreateFts5SyncTriggers` in its create tier, which is what makes every ordering
requirement fall out of one placement: the triggers come down before the FTS
table is dropped and before a content-table rebuild starts, and go back up only
once both tables exist again.

| Change | What the migration does |
|---|---|
| a new declaration | creates the three triggers and rebuilds; no `DROP TRIGGER`, which could not match anything |
| `sync_content()` added to an existing index | replaces the triggers only; the index is not recreated |
| `sync_content()` removed | drops the three triggers; the index keeps whatever it had indexed |
| the index dropped | drops the triggers first, then the table |
| tokenizer / prefix / column change | drop triggers → drop table → create table → create triggers → rebuild, exactly once |
| `content` or `content_rowid` change | replaces the triggers against the new source |
| a **content-table** column change | drops the triggers, rebuilds the content table, recreates the triggers, rebuilds the index |
| the index renamed | drops the old trigger names (`ALTER TABLE` renames the table, never its triggers) and creates the new ones |
| recreate **and** content rebuild in one diff | exactly one drop and one create per trigger |

The content-table case is the one #85 left open. A rebuild is
create-staging → copy → `DROP TABLE` → rename, and `DROP TABLE` takes every
trigger attached to the table with it; verified against libsql 0.9 /
SQLite 3.45.1, where the trigger count drops to zero. The index itself survives
untouched and the copy preserves the content rowid, so re-creating the triggers
would usually be enough — but the copy goes through the target column's type
affinity, which can rewrite the very text the index was built from, so the
migration re-runs `rebuild` as well. That is one re-index on an operation that
is already a full table copy.

### Refusals

A declaration the generator cannot turn into triggers is
`DbCoreError::Fts5SyncInvalid`, and `run_generate` writes no migration:

| Declaration | Reason |
|---|---|
| no `content` | `it sets no content table; a contentless index stores its own rows and needs no triggers` |
| `content = ''` | the same |
| no `content_rowid` | `it sets no content_rowid; the update trigger has to name that column to watch it` |
| no columns | `it declares no columns, so there is nothing to synchronize` |
| content table absent | `its content table "…" is not in the schema` |
| content table virtual | `its content table "…" is not an ordinary table` |
| content missing an FTS column | `its content table "…" is missing column "…"` |
| content missing the rowid column | `its content table "…" is missing content_rowid column "…"` |
| declared on a non-`fts5` table | `only an fts5 virtual table can synchronize a content table` |

The message ends with the way out: *Drop sync_content() to keep writing the
triggers by hand.*

### Against a real database

`crates/orm-cli/tests/fts5_sync_sqlite_test.rs` runs `generate → migrate` into
in-memory libsql and then writes to the **content** table — never to the index,
and never a manual `rebuild`. Besides the `MATCH` counts, every step ends with
`INSERT INTO memory_fts(memory_fts) VALUES('integrity-check')`, which is FTS5's
own verdict that the index and the content table agree; it is what would catch
a trigger writing the wrong values rather than none at all.
`crates/orm-connection/tests/fts5_sync_rusqlite_test.rs` applies the same
generated DDL through rusqlite, chunk by chunk, the way the migration runner
does.

### Postgres

FTS5 is SQLite-only, so both operations render a comment instead of DDL, folded
onto one line so a name cannot end it:

```
-- FTS5 synchronization triggers for "memory_fts" are SQLite-only; skipped for postgres
```

## How to run

```sh
cargo nextest run -p toolu-orm-core -E 'binary(fts5_sync_declaration_test) + binary(fts5_sync_sql_test) + binary(fts5_sync_diff_test) + binary(fts5_sync_rebuild_diff_test) + binary(fts5_sync_validation_test)'
cargo nextest run -p toolu-orm-macros -E 'binary(fts5_macro_test)'
cargo nextest run -p toolu-orm-cli -E 'binary(fts5_sync_sqlite_test) + binary(fts5_sync_recreate_sqlite_test)'
cargo nextest run -p toolu-orm-connection --features rusqlite,sqlite-vec -E 'binary(fts5_sync_rusqlite_test)'
```

## Tests

| Lane | Binary | Test |
|---|---|---|
| default | fts5_sync_declaration_test | sync_content_records_the_content_table_rowid_and_columns |
| default | fts5_sync_declaration_test | without_the_opt_in_nothing_is_recorded |
| default | fts5_sync_declaration_test | an_all_unindexed_table_records_no_indexed_columns |
| default | fts5_sync_declaration_test | a_missing_content_option_is_recorded_as_empty_rather_than_dropped |
| default | fts5_sync_declaration_test | trigger_names_key_on_the_fts_table_alone |
| default | fts5_sync_declaration_test | the_snapshot_round_trip_preserves_the_declaration |
| default | fts5_sync_declaration_test | a_table_without_the_declaration_writes_no_fts5_sync_key |
| default | fts5_sync_declaration_test | the_declaration_reaches_the_json_under_its_own_key |
| default | fts5_sync_declaration_test | a_snapshot_written_before_this_existed_loads_as_unsynchronized |
| default | fts5_sync_declaration_test | from_registry_carries_the_declaration_into_the_snapshot_table |
| default | fts5_sync_sql_test | the_insert_trigger_writes_every_column_addressed_by_the_content_rowid |
| default | fts5_sync_sql_test | the_delete_trigger_uses_the_fts5_delete_command_with_the_old_values |
| default | fts5_sync_sql_test | the_update_trigger_deletes_before_reinserting |
| default | fts5_sync_sql_test | the_update_trigger_is_armed_only_for_the_rowid_and_indexed_columns |
| default | fts5_sync_sql_test | an_all_unindexed_table_arms_the_update_trigger_on_the_rowid_alone |
| default | fts5_sync_sql_test | a_content_rowid_that_is_also_an_indexed_column_is_listed_once |
| default | fts5_sync_sql_test | creating_the_triggers_ends_with_the_rebuild_command |
| default | fts5_sync_sql_test | dropping_the_triggers_names_all_three_and_tolerates_their_absence |
| default | fts5_sync_sql_test | neither_name_can_end_a_statement |
| default | fts5_sync_sql_test | postgres_reports_the_skip_instead_of_emitting_trigger_ddl |
| default | fts5_sync_sql_test | a_newline_in_the_name_stays_inside_the_postgres_comment |
| default | fts5_sync_sql_test | recreating_a_synchronized_table_leaves_the_rebuild_to_the_trigger_operation |
| default | fts5_sync_diff_test | a_new_declaration_creates_the_triggers_and_rebuilds_without_a_drop |
| default | fts5_sync_diff_test | adding_the_declaration_to_an_existing_index_replaces_nothing_but_the_triggers |
| default | fts5_sync_diff_test | removing_the_declaration_drops_the_triggers_and_creates_none |
| default | fts5_sync_diff_test | dropping_the_index_drops_its_triggers_first |
| default | fts5_sync_diff_test | an_unchanged_schema_emits_nothing |
| default | fts5_sync_diff_test | a_tokenizer_change_recreates_the_table_then_its_triggers_then_the_index |
| default | fts5_sync_diff_test | changing_the_content_rowid_replaces_the_triggers |
| default | fts5_sync_diff_test | changing_the_indexed_columns_replaces_the_triggers |
| default | fts5_sync_rebuild_diff_test | a_content_table_rebuild_puts_the_triggers_back_and_reindexes |
| default | fts5_sync_rebuild_diff_test | a_content_table_rebuild_without_a_declaration_emits_no_triggers |
| default | fts5_sync_rebuild_diff_test | recreating_the_index_and_rebuilding_its_content_table_emits_one_pair |
| default | fts5_sync_rebuild_diff_test | renaming_the_index_drops_the_old_trigger_names_and_creates_the_new_ones |
| default | fts5_sync_rebuild_diff_test | two_indexes_over_one_content_table_get_their_own_triggers |
| default | fts5_sync_rebuild_diff_test | recreating_one_of_two_indexes_leaves_the_other_alone |
| default | fts5_sync_validation_test | a_declaration_with_no_content_table_is_refused |
| default | fts5_sync_validation_test | a_contentless_declaration_is_refused |
| default | fts5_sync_validation_test | a_declaration_without_content_rowid_is_refused |
| default | fts5_sync_validation_test | a_declaration_with_no_columns_is_refused |
| default | fts5_sync_validation_test | a_content_table_outside_the_schema_is_refused |
| default | fts5_sync_validation_test | a_virtual_content_table_is_refused |
| default | fts5_sync_validation_test | a_content_table_missing_an_fts_column_is_refused |
| default | fts5_sync_validation_test | a_content_table_missing_the_rowid_column_is_refused |
| default | fts5_sync_validation_test | a_declaration_on_a_table_that_is_not_fts5_is_refused |
| default | fts5_sync_validation_test | the_error_points_at_the_hand_written_alternative |
| default | fts5_sync_validation_test | a_valid_declaration_is_not_refused |
| default | fts5_macro_test | sync_content_records_the_declaration |
| default | fts5_macro_test | sync_content_false_and_an_absent_attribute_record_nothing |
| default | fts5_macro_test | the_declaration_does_not_reach_the_module_arguments |
| default | fts5_sync_sqlite_test | content_mutations_stay_indexed_without_a_manual_rebuild |
| default | fts5_sync_sqlite_test | an_unindexed_only_write_leaves_the_index_alone |
| default | fts5_sync_sqlite_test | a_rowid_change_moves_the_indexed_row_with_it |
| default | fts5_sync_recreate_sqlite_test | a_tokenizer_change_recreates_the_table_and_its_triggers |
| default | fts5_sync_recreate_sqlite_test | a_content_table_rebuild_puts_the_triggers_back |
| default | fts5_sync_recreate_sqlite_test | removing_the_declaration_leaves_the_index_without_triggers |
| default | fts5_sync_sqlite_test | an_invalid_declaration_writes_no_migration |
| rusqlite-only | fts5_sync_rusqlite_test | the_generated_ddl_creates_all_three_triggers |
| rusqlite-only | fts5_sync_rusqlite_test | an_insert_and_a_delete_reach_the_index_without_a_rebuild |
