# Embedded migrations

**Feature:** `run_migrate_embedded` applies migrations from a compile-time
`&[EmbeddedMigration]` — SQL baked in with `include_str!` — instead of reading a
migrations directory at runtime, so a single-binary distribution carries its
migrations inside the executable. Everything below the byte-fetch is the shared
`apply_migration` that `run_migrate` uses, so hashes, transaction boundaries and
rollback are the same on both paths.
**Drivers:** libsql (SQLite) and Postgres. rusqlite shares the SQLite path.
**Issue:** [#16](https://github.com/Falconiere/toolu-orm/issues/16).

## What is proven

The fixture is one two-statement migration (`CREATE_SQL`, split by
`--> statement-breakpoint`, creating `users` **and** `audit`) plus smaller
single-statement ones. `audit` is the witness that the statement *after* the
separator ran; `half`, from the deliberately failing migration, is the witness
that a rolled-back migration left nothing behind. Hashes are real
`compute_hash` output, and a "tampered" entry declares the hash of one body
while carrying another — the shipped `.sql` someone edited without re-hashing.

| Input | Observable result |
|---|---|
| a two-entry list against a fresh database | returns `2`; `users`, `audit` and `posts` exist; `_migrations` holds both names in list order with the declared hashes |
| the same list re-run | `0`; nothing executes |
| a third entry appended, re-run | `1`; only the new table is created |
| an entry whose SQL no longer matches its declared `hash` | `MigrateError::HashMismatch` naming it and reporting the declared hash as `expected`; its table does not exist and it is not recorded, while the valid entry before it stays applied |
| a list naming the same migration twice | `MigrateError::DuplicateMigration` naming it; **no** `_migrations` table is created and nothing runs — a hand-written array can repeat a name where a generated journal cannot |
| `&[]` | `0`; `_migrations` exists and is empty |
| an entry whose second statement fails | `MigrateError::Database` prefixed with the name; that migration's first statement is rolled back too; the entry before it stays applied and recorded |
| entries whose names sort the other way (`0009_make_t` before `0001_seed_t`) | applied in **list** order, so the insert finds its table; `_migrations.id` order matches the list — the list is the declaration of order, exactly as `_journal.json` is on disk |
| a database migrated by `run_migrate` from a directory, then handed the equivalent list | `0`; `_migrations` unchanged |
| a database migrated from the list, then handed the equivalent directory | `0` — the two sources are interchangeable, so a project can switch between releases |
| `EmbeddedMigration::verify_hash` on a matching and a mismatched pair | `Ok(())` / `HashMismatch`, with no database — how a project asserts its whole list in one test |

Already-applied entries are skipped without re-reading their hash, exactly as
`run_migrate` skips entries already in `_migrations`.

## How to run

```sh
cargo nextest run -p toolu-orm-cli -E 'binary(migrate_embedded_test)'
docker compose -f docker-compose.test.yaml up -d --wait
TEST_DB_PORT=5434 cargo nextest run -p toolu-orm-core -p toolu-orm-macros -p toolu-orm-query -p toolu-orm-connection -p toolu-orm-cli --features postgres -E 'binary(migrate_embedded_postgres_test)'
```

## Tests

| Lane | Binary | Test |
|---|---|---|
| default | migrate_embedded_test | an_embedded_list_applies_every_statement_of_every_migration |
| default | migrate_embedded_test | re_running_applies_nothing_and_a_new_entry_applies_alone |
| default | migrate_embedded_test | an_edited_migration_fails_the_hash_check_after_the_earlier_one_applied |
| default | migrate_embedded_test | a_repeated_name_is_rejected_before_anything_is_written |
| default | migrate_embedded_test | an_empty_list_still_creates_the_migrations_table |
| default | migrate_embedded_test | a_failing_statement_rolls_back_only_its_own_migration |
| default | migrate_embedded_test | the_list_order_wins_over_the_name_order |
| default | migrate_embedded_test | verify_hash_checks_a_migration_without_a_database |
| default | migrate_embedded_test | a_directory_migrated_database_accepts_the_equivalent_embedded_list |
| default | migrate_embedded_test | an_embedded_migrated_database_accepts_the_equivalent_directory |
| postgres | migrate_embedded_postgres_test | an_embedded_list_applies_and_records_every_migration_on_postgres |
| postgres | migrate_embedded_postgres_test | an_edited_migration_fails_the_hash_check_on_postgres |
| postgres | migrate_embedded_postgres_test | a_failing_statement_rolls_back_only_its_own_migration_on_postgres |
