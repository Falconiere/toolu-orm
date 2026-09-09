# Virtual tables (FTS5)

**Feature:** `TableKind::Virtual { module, args }` on `TableDef` models any SQLite virtual table; `Fts5Table` and `#[fts5_table]` build the FTS5 case; `create_table_sql` emits `CREATE VIRTUAL TABLE … USING <module>(…)`; in-place changes either rebuild from FTS5 external `content=` or refuse with `VirtualTableChange`.
**Drivers:** libsql and rusqlite (SQLite only). On Postgres the table is skipped with a comment.
**Spec:** local `docs/toolu/specs/2026-09-09-virtual-table-repopulate-design.md` (and #18 FTS5 design); AC-1 … AC-8 for rebuild.
**Reading one:** this page declares and creates the index; [FTS5 queries](fts5-queries.md) searches it with `MATCH` and ranks it with `bm25`.

## What is proven

### The shape of a virtual table

`TableKind` is a plain enum on `TableDef`, so a module other than FTS5 (`vec0`, `rtree`, `series`) needs no new type. `Fts5Table` renders the module arguments in a fixed order — columns first (each quoted, `UNINDEXED` appended when the column carries the flag), then `prefix`, `tokenize`, `content`, `content_rowid`, `columnsize`, `detail` — so the same schema always produces the same DDL and therefore an empty diff:

```
CREATE VIRTUAL TABLE IF NOT EXISTS "memory_fts" USING fts5(
  "memory_id" UNINDEXED, "body", "tags",
  tokenize = 'porter unicode61 remove_diacritics 2');
```

Column types, `STRICT`, `PRIMARY KEY`, `NOT NULL`, defaults and CHECKs never reach that DDL: FTS5 rejects them. Single quotes inside an option value are doubled.

### Against a real database

`crates/orm-cli/tests/fts5_loop_sqlite_test.rs` runs `generate → migrate` into in-memory libsql and then queries the live table; `crates/orm-connection/tests/fts5_rusqlite_test.rs` executes the same generated DDL through rusqlite. Both assert on the database, not on the SQL text:

| Assertion | How |
|---|---|
| the table is virtual | `sqlite_master.sql` contains `CREATE VIRTUAL TABLE … USING fts5` |
| full-text search works | `MATCH 'runner'` finds the row containing *running* (porter stemmer) |
| `UNINDEXED` is honoured | `MATCH 'zebrafish'` (indexed body) hits; `MATCH 'm1'` (unindexed `memory_id`) does not |
| the unindexed value is still stored | `SELECT memory_id … WHERE rowid = 1` returns it |
| ordinary tables are unaffected | `memories` is created next to `memory_fts` in the same migration |

### Evolving a virtual table

SQLite has no `ALTER TABLE` for virtual tables. Create, drop and rename stay
ordinary operations; a rename is not a way around the checks — a table renamed
*and* changed in the same diff is refused, and so is an ordinary table renamed
onto a virtual definition.

#### Rebuild from FTS5 external content

When the **new** FTS5 definition sets a non-empty `content = '…'` naming an
**ordinary** table in the same schema, every FTS column (and `content_rowid`,
when set) exists on that table, the module stays `fts5`, and the change is not
combined with a rename, `diff` emits `Operation::RecreateFts5FromContent`
instead of refusing. Generated SQLite SQL is drop + create + rebuild:

```
DROP TABLE IF EXISTS "memory_fts";
--> statement-breakpoint
CREATE VIRTUAL TABLE IF NOT EXISTS "memory_fts" USING "fts5"(…);
--> statement-breakpoint
INSERT INTO "memory_fts"("memory_fts") VALUES('rebuild');
```

Proven end to end by `fts5_content_rebuild_sqlite_test` (generate → migrate →
`MATCH` against rebuilt content). Postgres still emits the SQLite-only skip
comment and never runs `rebuild`.

Standalone FTS5 (no `content`, or `content = ''`), missing content tables,
missing columns, `vec0`, ordinary↔virtual, module switches, indexes, and
rename+change stay refused.

#### Refusing everything else

`diff` returns `DbCoreError::VirtualTableChange { table, reason }` and
`run_generate` writes no migration. The Display text also points at the
external-content rebuild path above. Reasons are ordered from the coarsest
difference to the finest (`crates/orm-core/tests/diff_test/virtual_table_diffs.rs`,
renames in `virtual_table_renames.rs`, recreate eligibility in
`virtual_table_recreate.rs`):

| Change | Reason |
|---|---|
| ordinary → virtual | `it became a virtual table using fts5` |
| virtual → ordinary | `it is no longer a virtual table using fts5` |
| `fts5` → `vec0` | `its module changed from fts5 to vec0` |
| a column added, or `UNINDEXED` flipped (no eligible content) | `its columns changed` |
| a different tokenizer, prefix, content… (no eligible content) | `its module arguments changed` |
| `content` table absent from the schema | `its content table "…" is not in the schema` |
| FTS column / `content_rowid` missing on content | `its content table "…" is missing column "…"` |
| an index declared on it | `virtual tables cannot declare indexes` |

The table and module names are quoted like any other identifier and their embedded quotes are doubled, so neither can end the statement; in the Postgres comment their line breaks are folded away, so neither can end the comment either.

### Old snapshots

`kind` and `unindexed` are `#[serde(default)]` and are skipped when they hold their default, so a snapshot written before this change loads as an ordinary table with indexed columns, and an ordinary table's JSON is byte-identical to what the previous version wrote — see [Legacy snapshot](legacy-snapshot.md).

### Postgres

`CREATE VIRTUAL TABLE` is SQLite-only. Generating for Postgres emits a comment naming the skipped table instead of invalid DDL, the same way an unsupported ALTER does:

```
-- virtual table "memory_fts" USING fts5 is SQLite-only; skipped for postgres
```

## How to run

```sh
cargo nextest run -p toolu-orm-core -E 'binary(virtual_table_test) + binary(diff_test) + binary(sql_test)'
cargo nextest run -p toolu-orm-macros -E 'binary(fts5_macro_test)'
cargo nextest run -p toolu-orm-cli -E 'binary(fts5_loop_sqlite_test) + binary(fts5_content_rebuild_sqlite_test)'
cargo nextest run -p toolu-orm-connection --features rusqlite -E 'binary(fts5_rusqlite_test)'
```

## Tests

| Lane | Binary | Test |
|---|---|---|
| default | virtual_table_test | builder_renders_columns_then_options |
| default | virtual_table_test | builder_marks_only_the_unindexed_column |
| default | virtual_table_test | builder_renders_every_option_in_a_fixed_order |
| default | virtual_table_test | builder_escapes_single_quotes_in_option_values |
| default | virtual_table_test | sqlite_ddl_creates_the_virtual_table |
| default | virtual_table_test | sqlite_ddl_omits_types_strict_and_constraints |
| default | virtual_table_test | a_module_without_arguments_omits_the_parentheses |
| default | virtual_table_test | neither_name_can_end_the_statement |
| default | virtual_table_test | postgres_reports_the_skipped_table_instead_of_emitting_ddl |
| default | virtual_table_test | a_newline_in_a_name_stays_inside_the_skipped_table_comment |
| default | virtual_table_test | snapshot_round_trip_keeps_the_module_arguments |
| default | virtual_table_test | ordinary_tables_write_no_kind_key |
| default | fts5_macro_test | table_def_is_a_virtual_fts5_table |
| default | fts5_macro_test | only_the_marked_column_is_unindexed |
| default | fts5_macro_test | every_option_reaches_the_module_arguments |
| default | fts5_macro_test | the_column_module_is_generated |
| default | fts5_macro_test | the_builder_factories_are_generated |
| default | fts5_loop_sqlite_test | migrate_creates_the_virtual_table |
| default | fts5_loop_sqlite_test | the_created_table_answers_a_match_query |
| default | fts5_loop_sqlite_test | the_unindexed_column_is_stored_but_not_searchable |
| default | fts5_loop_sqlite_test | regenerating_the_same_schema_finds_no_change |
| default | fts5_loop_sqlite_test | changing_the_tokenizer_is_refused_without_writing_a_migration |
| default | fts5_content_rebuild_sqlite_test | retokenizing_external_content_fts_rebuilds_from_the_content_table |
| rusqlite-only | fts5_rusqlite_test | generated_ddl_creates_a_virtual_table |
| rusqlite-only | fts5_rusqlite_test | match_finds_the_stemmed_row |
| rusqlite-only | fts5_rusqlite_test | the_unindexed_column_is_stored_but_not_searchable |
