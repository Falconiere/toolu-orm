# Migration loop

**Feature:** `run_generate` diffs a `SchemaRegistry` against the last snapshot and writes `NNNN_name.sql` + snapshot + journal; `run_migrate` applies pending files in one transaction each; `get_status` lists applied and pending.
**Drivers:** libsql (SQLite) and Postgres. rusqlite shares the SQLite DDL path.
**Spec:** AC-6, AC-7.

## What is proven

Registries come from real `#[table]` structs (`crates/orm-cli/tests/fixtures/loop_registry.rs`):

- **v1:** `users(id TEXT PK, name TEXT NOT NULL, email TEXT NOT NULL, age INTEGER)`.
- **v2:** `users` gains `bio`; a unique index `idx_users_email`; a new `posts` table with `author_id REFERENCES users(id) ON DELETE CASCADE` and `status: Status` (a `#[derive(ColumnEnum)]`, stored as `TEXT` + `CHECK (status IN ('active', 'banned'))`).

The loop `generate(v1) → migrate → generate(v2) → migrate` is asserted on the live database, not on the SQL text:

| Assertion | SQLite (libsql) | Postgres |
|---|---|---|
| `users` columns after v1 | `PRAGMA table_info(users)` = id, name, email, age | `information_schema.columns` count = 4 |
| `bio` added, `posts` created | `PRAGMA table_info`, `sqlite_master` | `information_schema.columns` / `.tables` |
| unique index exists | `sqlite_master WHERE type = 'index'` | `pg_indexes` |
| FK exists | `pragma_foreign_key_list('posts')` | `information_schema.table_constraints` (`FOREIGN KEY`) |
| enum CHECK enforced | `INSERT ... status = 'bogus'` fails | `INSERT ... status = 'bogus'` fails; `information_schema.check_constraints` has it |
| unique index enforced | duplicate email insert fails | duplicate email insert fails |
| `ON DELETE CASCADE` | deleting the user removes the post (`PRAGMA foreign_keys = ON`) | (covered by the FK constraint assertion) |
| `get_status` | applied = 2 files, pending = 0 | same |
| nothing left to generate | third `run_generate` returns `None` | same |
| legacy (no journal) mode | see [Migration failures](migration-failures.md) | applies a plain `.sql` file as one batch, records it with an empty hash |

### Bugs this loop caught

- **Foreign keys on non-strict tables were dropped.** `column_def_sql` only rendered `REFERENCES ... ON DELETE` when the table was `strict`, so every FK declared with `#[column(references = ...)]` on a default table vanished from `CREATE TABLE` on both dialects. Fixed in `crates/orm-core/src/sql/ddl.rs`.
- **Comment-only migration chunks failed on libsql.** The SQLite generator emits only a `--` comment for "add CHECK constraint"; the runner handed it to the driver, which answered `SQLite failure: 'not an error'`. `run_migrate` now skips chunks with no statement (`crates/orm-cli/src/migrate/run.rs`).

### Known parity gap

Adding a CHECK (an enum column) to an **existing** table is still a no-op comment on SQLite; a table rebuild carrying the new `TableDef` is needed. That is why the enum column lives on the new `posts` table in this scenario. Tracked as spec Q7.

## How to run

```sh
cargo nextest run -p toolu-orm-cli -E 'binary(migration_loop_sqlite_test)'
docker compose -f docker-compose.test.yaml up -d --wait
TEST_DB_PORT=5434 cargo nextest run -p toolu-orm-core -p toolu-orm-macros -p toolu-orm-query -p toolu-orm-connection -p toolu-orm-cli --features postgres -E 'binary(migration_loop_postgres_test)'
```

## Tests

| Lane | Binary | Test |
|---|---|---|
| default | migration_loop_sqlite_test | loop_v1_then_v2_applies_every_change |
| default | migration_loop_sqlite_test | evolved_schema_enforces_enum_check_and_unique_email |
| default | migration_loop_sqlite_test | evolved_schema_cascades_post_deletes |
| postgres | migration_loop_postgres_test | loop_v1_then_v2_applies_every_change_on_postgres |
| postgres | migration_loop_postgres_test | mid_file_failure_rolls_back_on_postgres |
| postgres | migration_loop_postgres_test | legacy_mode_applies_plain_sql_files_on_postgres |
| postgres | status_test | test_status_shows_applied_and_pending |
| postgres | status_test | status_reports_applied_and_pending |
