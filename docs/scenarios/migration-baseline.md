# Migration baseline

**Feature:** `mark_applied` / `mark_applied_through` record journal entries as applied **without executing their SQL**, so a database whose schema was built by a previous migration system can adopt toolu-orm instead of re-running every migration. Hashes come from `_journal.json`, so the tamper check keeps working from the baseline onward.
**Drivers:** libsql (SQLite) and Postgres for the async path; rusqlite via `mark_applied*_blocking` (see [Blocking connection](blocking-connection.md)).
**Issue:** [#14](https://github.com/Falconiere/toolu-orm/issues/14).

## What is proven

The fixture is the adoption case itself: a database that already carries `users`
(put there by a prior runner) and a migrations directory whose `0001_init.sql`
would create `users` **and** a second table `audit`. `audit` is the witness — if
any statement of a baselined file ran, it would exist.

| Input | Observable result |
|---|---|
| no baseline, `run_migrate` on the adopted database | `MigrateError::Database` (`users` already exists), `_migrations` empty — the problem this feature solves |
| `mark_applied(&["0001_init.sql"])` | returns `1`; one `_migrations` row whose `hash` equals `compute_hash` of the file, i.e. the journal hash; `audit` does not exist |
| a second file generated, then `run_migrate` | returns `1`; only `0002_*` applied; `audit` still absent; `get_status` = both applied, nothing pending |
| `mark_applied(&["0001_init.sql", "0009_ghost.sql"])` | `MigrateError::NotInJournal` naming the ghost; **no** `_migrations` table is created; baselining the valid name afterwards still returns `1` |
| `mark_applied_through("0009_ghost.sql")` | same `NotInJournal` rejection |
| the same names baselined twice (and a repeated name in one call) | `1` then `0`; exactly one row — `_migrations.name` is `UNIQUE`, so already-recorded names are skipped |
| `mark_applied(&[])` | `0`; `_migrations` exists, `get_status` reports the file as pending |
| `mark_applied_through("0002_…")` on a three-entry journal | `2`, recorded in journal order; `0003` stays pending and applies on the next `run_migrate`; `posts` from `0002` was never created |
| a pending file edited after a baseline | `run_migrate` still fails with `MigrateError::HashMismatch` for that file |
| a journal entry whose `.sql` file was deleted | baselined normally — a baseline never reads the migration files |

A baselined file that is later edited is **not** detected, exactly as for a file
applied the normal way: `run_migrate` skips entries already in `_migrations`.

## How to run

```sh
cargo nextest run -p toolu-orm-cli -E 'binary(migrate_baseline_test)'
docker compose -f docker-compose.test.yaml up -d --wait
TEST_DB_PORT=5434 cargo nextest run -p toolu-orm-core -p toolu-orm-macros -p toolu-orm-query -p toolu-orm-connection -p toolu-orm-cli --features postgres -E 'binary(migrate_baseline_postgres_test)'
```

## Tests

| Lane | Binary | Test |
|---|---|---|
| default | migrate_baseline_test | migrate_without_a_baseline_fails_on_an_adopted_database |
| default | migrate_baseline_test | baseline_records_the_journal_hash_without_running_the_file |
| default | migrate_baseline_test | migrate_after_a_baseline_applies_only_the_later_files |
| default | migrate_baseline_test | a_name_absent_from_the_journal_records_nothing |
| default | migrate_baseline_test | repeating_a_baseline_records_nothing_new |
| default | migrate_baseline_test | an_empty_baseline_still_creates_the_migrations_table |
| default | migrate_baseline_test | mark_applied_through_baselines_the_prefix_and_leaves_the_rest_pending |
| default | migrate_baseline_test | a_pending_file_edited_after_a_baseline_still_fails_the_hash_check |
| default | migrate_baseline_test | a_journal_entry_with_no_file_on_disk_is_baselined |
| postgres | migrate_baseline_postgres_test | baseline_then_migrate_skips_the_baselined_entry_on_postgres |
| postgres | migrate_baseline_postgres_test | a_name_absent_from_the_journal_records_nothing_on_postgres |
| postgres | migrate_baseline_postgres_test | mark_applied_through_baselines_the_prefix_on_postgres |
