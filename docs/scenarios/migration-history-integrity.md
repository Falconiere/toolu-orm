# Migration history integrity

**Feature:** every runner validates the migrations a database has **already applied** before it skips them. `_migrations` keeps the hash each migration was applied with; a run compares it with the hash the journal (or the embedded list) declares now, and that hash with the migration's current bytes. A history that no longer matches fails before any pending migration executes.
**Drivers:** libsql (SQLite) for the async runners, rusqlite for the blocking twins. The logic is dialect-free: it compares hashes the runner already read.
**Spec:** issue #86.

## What is proven

| Input | Observable result |
|---|---|
| an applied `.sql` file edited afterwards, journal untouched | `MigrateError::HashMismatch { file, expected, actual }` — `expected` is the journal hash, `actual` the edited bytes' hash; `_migrations` unchanged |
| the same edit under `run_migrate_blocking` | same `HashMismatch` |
| an applied entry whose journal hash was rewritten (file rewritten to match) | `MigrateError::HistoryMismatch { file, recorded, declared }` — `recorded` is what the database applied, `declared` what the journal claims now |
| an applied embedded entry whose `sql` changed | `HashMismatch`, on both the async and the blocking embedded runner |
| an applied embedded entry whose declared `hash` changed | `HistoryMismatch`, on both |
| a tampered applied entry followed by a genuinely pending one | the run fails, `_migrations` gains no row, and the pending migration's table is not created |
| a row recorded with an empty hash (journal-free runner, or a database older than the hash column), with the file edited and a journal added since | `Ok(0)` — unverifiable, so skipped without a verdict rather than reported as verified |
| an applied name the journal no longer lists (squashed history), its file edited | `Ok(0)` — the source declares nothing to compare |
| an applied entry whose `.sql` file was pruned from disk, journal hash unchanged | `Ok(0)` — the recorded-against-declared comparison is the whole verdict |
| an applied entry pruned from disk whose journal hash was rewritten | `HistoryMismatch` — pruning does not excuse a rewritten history |
| an applied entry whose path exists but cannot be read (a directory in its place) | `MigrateError::ReadFile` naming the path — never mistaken for a pruned file |
| an unchanged journaled directory, or an unchanged embedded list, run twice | `Ok(n)` then `Ok(0)` |
| a migration adopted with `mark_applied` (never executed) whose file is edited afterwards | `HashMismatch` — a baseline records the journal's hash, so baselined history is tamper-evident too |

The empty-hash and pruned-file rows are the deliberate limits of the check:
`_migrations.hash` is `NOT NULL DEFAULT ''`, so rows that predate hashes — and
rows the journal-free path still writes — carry nothing to compare, and a
project that squashes its history or adopts toolu-orm with `mark_applied` may
legitimately no longer have every `.sql` on disk.

## How to run

```sh
cargo nextest run -p toolu-orm-cli -E 'binary(migrate_history_test) + binary(migrate_history_embedded_test)'
cargo nextest run -p toolu-orm-cli --no-default-features --features rusqlite -E 'binary(migrate_history_blocking_test)'
```

## Tests

| Lane | Binary | Test |
|---|---|---|
| default | migrate_history_test | editing_an_applied_file_fails_the_next_run |
| default | migrate_history_test | a_tampered_entry_blocks_a_pending_migration |
| default | migrate_history_test | rewriting_an_applied_journal_hash_is_a_history_mismatch |
| default | migrate_history_test | a_row_recorded_without_a_hash_is_skipped_unverified |
| default | migrate_history_test | a_journal_entry_dropped_from_a_pruned_history_is_not_validated |
| default | migrate_history_test | an_applied_file_pruned_from_disk_passes_on_the_recorded_hash |
| default | migrate_history_test | a_pruned_file_whose_journal_hash_changed_still_fails |
| default | migrate_history_test | an_unreadable_applied_file_is_a_read_file_error |
| default | migrate_history_test | a_baselined_migration_is_still_tamper_evident |
| default | migrate_history_test | an_unchanged_history_stays_idempotent |
| default | migrate_history_embedded_test | an_edited_body_for_an_applied_name_fails |
| default | migrate_history_embedded_test | a_rehashed_entry_for_an_applied_name_is_a_history_mismatch |
| default | migrate_history_embedded_test | a_tampered_entry_blocks_a_pending_embedded_entry |
| default | migrate_history_embedded_test | an_unchanged_embedded_list_stays_idempotent |
| rusqlite-only | migrate_history_blocking_test | editing_an_applied_file_fails_the_next_blocking_run |
| rusqlite-only | migrate_history_blocking_test | rewriting_an_applied_journal_hash_fails_the_next_blocking_run |
| rusqlite-only | migrate_history_blocking_test | an_unchanged_directory_history_stays_idempotent_blocking |
| rusqlite-only | migrate_history_blocking_test | an_edited_embedded_body_fails_the_next_blocking_run |
| rusqlite-only | migrate_history_blocking_test | a_rehashed_embedded_entry_fails_the_next_blocking_run |
