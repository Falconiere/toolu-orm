# Migration failures

**Feature:** `run_migrate` applies each file inside `BEGIN ... COMMIT`; any error rolls the whole file back and records nothing. Inputs it cannot read are reported as typed `MigrateError`s, never applied half-way.
**Drivers:** libsql (SQLite); the Postgres rollback case lives in [Migration loop](migration-loop.md).
**Spec:** AC-8, AC-9.

## What is proven

| Input | Observable result |
|---|---|
| journal mode, file `CREATE TABLE a (...); --> statement-breakpoint INSERT INTO nope VALUES (1);` | `MigrateError::Database` naming the file; `_migrations` stays empty; table `a` does not exist |
| legacy mode (no `_journal.json`), same two statements in one file | same rollback |
| `migrations_dir` is a regular file | `MigrateError::ReadDir` |
| journal names `0001_missing.sql` that is not on disk | `MigrateError::ReadFile` naming the file |
| `_journal.json` contains `{not json` | `MigrateError::ReadFile` naming the journal |
| `migrations_dir` does not exist | `Ok(0)`, `_migrations` empty |
| a breakpoint chunk that is only a `--` comment | skipped; the surrounding statements apply and the file is recorded once |
| a file whose content no longer matches its journal `sha256:` hash | `MigrateError::HashMismatch { file, expected, actual }`, nothing applied (`migrate_test`) |
| the same directory migrated twice | second run applies 0 |
| `_migrations` DDL per dialect | `ensure_migrations_table` creates the dialect-appropriate table |

The comment-only case is the regression test for a bug this program found: the SQLite generator emits a comment for "add CHECK constraint", and libsql answers `SQLite failure: 'not an error'` when asked to execute an empty statement.

## How to run

```sh
cargo nextest run -p toolu-orm-cli -E 'binary(migrate_failure_test)'
docker compose -f docker-compose.test.yaml up -d --wait
TEST_DB_PORT=5434 cargo nextest run -p toolu-orm-core -p toolu-orm-macros -p toolu-orm-query -p toolu-orm-connection -p toolu-orm-cli --features postgres -E 'binary(migrate_test)'
```

## Tests

| Lane | Binary | Test |
|---|---|---|
| default | migrate_failure_test | journal_mode_failing_statement_rolls_back_whole_file |
| default | migrate_failure_test | legacy_mode_failing_statement_rolls_back_whole_file |
| default | migrate_failure_test | migrations_dir_that_is_a_file_is_read_dir_error |
| default | migrate_failure_test | journal_entry_without_sql_file_is_read_file_error |
| default | migrate_failure_test | malformed_journal_is_read_file_error |
| default | migrate_failure_test | absent_migrations_dir_applies_nothing |
| default | migrate_failure_test | comment_only_breakpoint_chunk_is_skipped |
| postgres | migrate_test | test_migrate_applies_pending_migrations_legacy |
| postgres | migrate_test | test_migrate_skips_already_applied |
| postgres | migrate_test | test_migrate_with_journal_and_breakpoints |
| postgres | migrate_test | test_migrate_hash_mismatch_errors |
| postgres | migrate_test | ensure_migrations_table_creates_correct_schema_for_dialect |
