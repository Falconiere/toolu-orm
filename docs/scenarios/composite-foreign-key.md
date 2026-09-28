# Composite foreign key

**Feature:** `#[foreign_key(name = "…", columns(a, b), references = "t(x, y)", on_delete = "…", on_update = "…")]`
on a `#[table]` declares a table-level foreign key over two or more columns,
stored in `TableDef.foreign_keys` and snapshotted beside the column-level keys.
SQLite renders it as a trailing `FOREIGN KEY (…) REFERENCES … (…)` clause in
`CREATE TABLE` and in the table rebuild; Postgres attaches it with
`ALTER TABLE … ADD CONSTRAINT "<name>" FOREIGN KEY …` after every table and
index of the migration, because Postgres checks the referenced unique key when
the constraint is created. Adding, removing or changing one on an existing
SQLite table rebuilds the table.
**Drivers:** SQLite (real libsql) and live Postgres for execution; both
dialects for SQL generation.
**Spec:** issue #264.

## What is proven

| Shape | Proof |
|---|---|
| Self-reference `(parent_work_item_id, project_id)` and cascading `(work_item_id, project_id)` | Migrates on libsql and Postgres; a pair naming the wrong project is refused (SQLite `FOREIGN KEY constraint failed`, Postgres SQLSTATE 23503) |
| `ON DELETE CASCADE` / no action | Deleting a work item removes its evidence; deleting one that still has a child is refused |
| NULL member | A NULL `parent_work_item_id` skips the key (SQLite `MATCH SIMPLE`) while a non-NULL dangling pair is still refused |
| Adding a key to a populated table | libsql rebuilds `project_evidence` through `_toolu_new_project_evidence` and keeps its rows; Postgres adds the constraint in place |
| Snapshot | A registry with composite keys written and read back diffs to nothing; a snapshot from before table-level keys still loads (`snapshot_test`) |
| Macro | Declarations land in `TableDef.foreign_keys` (`table_macro_test`); eight malformed declarations fail with pinned messages (`compile_fail_test`, see [Macro compile errors](macro-compile-errors.md)) |

A column may carry its own `#[column(references = …)]` and also be a member
of a composite key, as `project_id` is in the suites.

## How to run

```sh
cargo nextest run -p toolu-orm-cli -E 'binary(composite_fk_sqlite_test)'
docker compose -f docker-compose.test.yaml up -d --wait
TEST_DB_PORT=5434 cargo nextest run -p toolu-orm-cli --features postgres -E 'binary(composite_fk_postgres_test)'
cargo nextest run -p toolu-orm-core -E 'binary(snapshot_test) + binary(sql_test) + binary(diff_test)'
cargo nextest run -p toolu-orm-macros --features postgres -E 'binary(table_macro_test) + binary(compile_fail_test)'
```

## Tests

| Lane | Binary | Test |
|---|---|---|
| default | composite_fk_sqlite_test | composite_fks_render_and_refuse_dangling_pairs |
| default | composite_fk_sqlite_test | null_member_skips_the_composite_fk |
| default | composite_fk_sqlite_test | no_action_refuses_the_parent_delete_and_cascade_removes_children |
| default | composite_fk_sqlite_test | adding_a_composite_fk_rebuilds_the_populated_table |
| postgres | composite_fk_postgres_test | composite_fks_migrate_and_hold_on_postgres |
| postgres | composite_fk_postgres_test | adding_a_composite_fk_attaches_a_constraint_on_postgres |
