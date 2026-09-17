# SQLite table rebuild

**Feature:** SQLite cannot change a column's type, default, nullability,
uniqueness or primary key in place, so `generate_sql_for(.., Dialect::Sqlite)`
rebuilds the table. The rebuild follows
[SQLite's documented procedure](https://sqlite.org/lang_altertable.html) —
create a staging table, copy, drop the old table, rename the staging table into
place — and the migration runner applies the foreign-key pragma *around* the
transaction, where it is not a no-op.
**Drivers:** libsql (SQLite) for the async path; rusqlite via
`run_migrate_blocking` for the blocking twin.
**Spec:** issue #85.

## What the generator emits

For each table a diff forces SQLite to rebuild, one chunk sequence:

```sql
PRAGMA foreign_keys = OFF;                       -- hoisted by the runner
CREATE TABLE "_toolu_new_users" ( … target definition … );
INSERT INTO "_toolu_new_users" ("id", …) SELECT "id", … FROM "users";
DROP TABLE "users";
PRAGMA legacy_alter_table = ON;
ALTER TABLE "_toolu_new_users" RENAME TO "users";
PRAGMA legacy_alter_table = OFF;
CREATE UNIQUE INDEX IF NOT EXISTS "idx_users_email" ON "users" ("email");
```

Why each piece is the way it is:

- **Staging table first, rename last.** Renaming the *old* table out of the way
  rewrites every child table's `REFERENCES` clause — since SQLite 3.26 that
  happens whether or not foreign keys are enforced — and the following
  `DROP TABLE` then runs the child's `ON DELETE` action. That is the data loss
  issue #85 reproduced.
- **No `IF NOT EXISTS` on the staging table.** A name that is already taken must
  fail the migration, not be silently adopted.
- **The pragma is a request to the runner.** SQLite documents
  `PRAGMA foreign_keys` as a no-op inside a transaction, and the runner opens
  one before the migration's first statement. `crates/orm-cli/src/migrate/pragma_guard.rs`
  therefore reads the setting before `BEGIN`, switches it off there, runs
  `PRAGMA foreign_key_check` before the commit when the connection was
  enforcing them, and restores the original value after the transaction ends —
  after a commit, after a rollback, and after a failed rollback.
- **`legacy_alter_table` around the rename.** On the SQLite generation libsql
  and rusqlite bundle (3.45), a plain rename re-parses every view and trigger
  and fails on any that names the table the rename is about to restore. Legacy
  mode skips that re-parse, and it is also the semantics a rebuild wants:
  nothing references the staging name, and everything that references the final
  name is already correct. The runner saves and restores this pragma too.
- **One rebuild per table.** `plan_sqlite_rebuilds` folds every `AlterColumn`,
  `AddColumn` and `DropColumn` on a rebuilt table into a single rebuild carrying
  the whole target definition, so alter+add and alter+drop apply in one pass.
  Index operations are left alone: `DropIndex` is ordered before the rebuild and
  `CreateIndex` after it is a no-op, because the rebuild re-creates every index
  the table declares — including the ones the diff never mentioned.

## What is proven

Registries come from real `#[table]` structs
(`crates/orm-cli/tests/fixtures/rebuild_registry.rs`): `users` with a unique
index on `email` and a nullable `name`, and `posts` whose `author_id`
references `users(id)`. Every assertion is made on the live database through
`PRAGMA` and `sqlite_master`, never on the generated SQL.

| Assertion | How |
|---|---|
| Cascading child rows survive | `PRAGMA foreign_keys = ON`, one user and one post, `users.name` → NOT NULL; `posts` still has its row |
| The child's FK still points at `users` | `pragma_foreign_key_list('posts')` names `users`, not a staging or `_old` table |
| Referential integrity is intact | `pragma_foreign_key_check` is empty |
| A non-cascading child is not a constraint failure | same loop with a plain `REFERENCES users(id)` |
| An untouched index comes back | `sqlite_master` has `idx_users_email`, and a duplicate email is still rejected |
| Foreign keys end where they started | asserted for a connection with them on, and for one with them off |
| A view and a trigger on a related table survive | the view still resolves; the trigger still updates the rebuilt table |
| Alter + add in one diff | `age` arrives with its declared default on the pre-existing row, `users` exists once, no staging table is left |
| Alter + drop in one diff | `bio` is gone, the row survives, `name` is now NOT NULL |
| A failed copy changes nothing | a row with a NULL `name` makes the NOT NULL rebuild fail: the table, its columns, its rows, `_migrations` and the pragma are all as they were |
| A journal-free directory is guarded too | a hand-written rebuild applied through the legacy (no `_journal.json`) path keeps the child row and restores the pragma |
| An orphan fails the migration | a hand-written embedded migration that deletes a parent gets `MigrateError::ForeignKeyViolation`, is rolled back whole, and foreign keys come back on |
| The blocking runner behaves the same | the reproduction and the pragma round trip on rusqlite |

## Limitations

- **Triggers attached to the rebuilt table are dropped** with it, and only the
  ones the schema models are replayed. The single modelled kind is FTS5
  synchronization: when a rebuilt table is the content table of an index that
  declared `sync_content()`, the migration drops those triggers before the
  rebuild, recreates them after it, and re-runs `rebuild` — see
  [FTS5 synchronization triggers](fts5-sync-triggers.md). Hand-written triggers
  on the rebuilt table are still lost, because there is nothing to replay them
  from. Triggers on *other* tables and views over the rebuilt table are
  unaffected, because the table keeps its name throughout.
- **Indexes that are not declared on the `#[table]` struct are lost**, for the
  same reason. This was already true of every generated migration.
- **Migrations generated before this fix are not rewritten** — their journal
  hashes pin them. A database that already ran a broken rebuild needs its lost
  child rows restored from a backup, and a rebuild of the *child* table to move
  its foreign key off the dropped `_<table>_old`.
- A migration that leans on `ON DELETE CASCADE` to clean up rows *and* carries a
  `foreign_keys` pragma no longer cascades; the orphans it leaves are reported
  by `foreign_key_check` and the migration fails instead of half-applying.

## How to run

```sh
cargo nextest run -p toolu-orm-cli -E 'binary(sqlite_rebuild_libsql_test) + binary(sqlite_rebuild_column_sets_libsql_test) + binary(sqlite_rebuild_rollback_libsql_test)'
cargo nextest run -p toolu-orm-cli --no-default-features --features rusqlite -E 'binary(sqlite_rebuild_blocking_test)'
cargo nextest run -p toolu-orm-core -E 'binary(sql_test)'
```

## Tests

| Lane | Binary | Test |
|---|---|---|
| default | sqlite_rebuild_libsql_test | rebuild_keeps_cascading_child_rows_and_their_fk_target |
| default | sqlite_rebuild_libsql_test | rebuild_keeps_a_non_cascading_child_row |
| default | sqlite_rebuild_libsql_test | rebuild_restores_an_index_the_diff_never_mentioned |
| default | sqlite_rebuild_libsql_test | a_rebuild_leaves_foreign_keys_the_way_it_found_them |
| default | sqlite_rebuild_libsql_test | a_view_and_a_related_trigger_survive_the_rebuild |
| default | sqlite_rebuild_column_sets_libsql_test | altering_and_adding_a_column_applies_in_one_rebuild |
| default | sqlite_rebuild_column_sets_libsql_test | altering_and_dropping_a_column_applies_in_one_rebuild |
| default | sqlite_rebuild_rollback_libsql_test | a_copy_that_violates_not_null_rolls_the_whole_rebuild_back |
| default | sqlite_rebuild_rollback_libsql_test | an_orphaned_row_fails_the_migration_and_restores_foreign_keys |
| default | sqlite_rebuild_rollback_libsql_test | a_journal_free_rebuild_is_guarded_the_same_way |
| rusqlite-only | sqlite_rebuild_blocking_test | blocking_rebuild_keeps_cascading_child_rows_and_their_fk_target |
| rusqlite-only | sqlite_rebuild_blocking_test | blocking_rebuild_leaves_foreign_keys_off_when_they_started_off |
