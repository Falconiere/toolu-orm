# Virtual tables (FTS5)

**Feature:** `TableKind::Virtual { module, args }` on `TableDef` models any SQLite virtual table; `Fts5Table` and `#[fts5_table]` build the FTS5 case; `create_table_sql` emits `CREATE VIRTUAL TABLE … USING <module>(…)`; the diff refuses every in-place change to a virtual table.
**Drivers:** libsql and rusqlite (SQLite only). On Postgres the table is skipped with a comment.
**Spec:** [FTS5 virtual tables](../toolu/specs/2026-09-07-fts5-virtual-tables-design.md), AC-1 … AC-9.

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

### Refusing to evolve one in place

SQLite has no `ALTER TABLE` for virtual tables, so `diff` returns `DbCoreError::VirtualTableChange { table, reason }` instead of emitting an operation nobody can apply. `run_generate` propagates it and writes no migration file. The reasons are ordered from the coarsest difference to the finest (`crates/orm-core/tests/diff_test/virtual_table_diffs.rs`):

| Change | Reason |
|---|---|
| ordinary → virtual | `it became a virtual table using fts5` |
| virtual → ordinary | `it is no longer a virtual table using fts5` |
| `fts5` → `vec0` | `its module changed from fts5 to vec0` |
| a column added, or `UNINDEXED` flipped | `its columns changed` |
| a different tokenizer, prefix, content… | `its module arguments changed` |
| an index declared on it | `virtual tables cannot declare indexes` |

Create, drop and rename stay ordinary operations: `RenameTable` still works, and the fix for a refused change is to drop and recreate the table.

### Old snapshots

`kind` and `unindexed` are `#[serde(default)]` and are skipped when they hold their default, so a snapshot written before this change loads as an ordinary table with indexed columns, and an ordinary table's JSON is byte-identical to what the previous version wrote — see [Legacy snapshot](legacy-snapshot.md).

### Postgres

`CREATE VIRTUAL TABLE` is SQLite-only. Generating for Postgres emits a comment naming the skipped table instead of invalid DDL, the same way an unsupported ALTER does:

```
-- virtual table "memory_fts" USING fts5 is SQLite-only; skipped for postgres
```

## How to run

```sh
cargo nextest run -p toolu-orm-core -E 'binary(virtual_table_test) + binary(diff_test)'
cargo nextest run -p toolu-orm-macros -E 'binary(fts5_macro_test)'
cargo nextest run -p toolu-orm-cli -E 'binary(fts5_loop_sqlite_test)'
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
| default | virtual_table_test | postgres_reports_the_skipped_table_instead_of_emitting_ddl |
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
| rusqlite-only | fts5_rusqlite_test | generated_ddl_creates_a_virtual_table |
| rusqlite-only | fts5_rusqlite_test | match_finds_the_stemmed_row |
| rusqlite-only | fts5_rusqlite_test | the_unindexed_column_is_stored_but_not_searchable |
