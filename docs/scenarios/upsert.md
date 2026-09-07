# Upsert and plain mutations

**Feature:** `InsertBuilder` / `UpdateBuilder` / `DeleteBuilder` share `.execute(&exec)` and return the affected-row count. `InsertBuilder::or_replace()` and `or_ignore()` choose the conflict mode; `conflict_columns(&[...])` names the Postgres conflict target (defaults to the first inserted column; ignored on SQLite).
**Drivers:** libsql, rusqlite, Postgres.
**Spec:** AC-4 (upsert), AC-3 (Postgres CRUD).

## SQL per dialect

| Builder | SQLite (libsql, rusqlite) | Postgres |
|---|---|---|
| `insert().or_replace().conflict_columns(&["id"])` | `INSERT OR REPLACE INTO "users" (...) VALUES (?1, ...)` | `INSERT INTO "users" (...) VALUES ($1, ...) ON CONFLICT ("id") DO UPDATE SET "name" = EXCLUDED."name", ...` |
| `insert().or_ignore()` | `INSERT OR IGNORE INTO ...` | `INSERT INTO ... ON CONFLICT DO NOTHING` |
| `update().set(&NAME, "x").set_expr(&AGE, "\"age\" + 1").filter(ID.eq("u1"))` | `UPDATE "users" SET "name" = ?1, "age" = "age" + 1 WHERE "users"."id" = ?2` | same with `$N` |
| `delete().filter(ID.eq("u1"))` | `DELETE FROM "users" WHERE "users"."id" = ?1` | same with `$1` |

## What is proven

- Inserting `("u1", "Ann")` then `("u1", "Bea")` with `or_replace` leaves exactly one row named `Bea` on all three drivers; Postgres reports 1 affected row for the `DO UPDATE`.
- The same with `or_ignore` keeps `Ann`; Postgres reports 0 affected rows.
- Postgres: insert returns 1, `update` with `set` + `set_expr` changes the row (`age` 30 to 31), `delete` returns 1 and the table is empty afterwards. The SQLite CRUD path is covered by the revived `executor_test` and `integration_test` (see [Lanes](lanes.md)).

## How to run

```sh
cargo nextest run -p toolu-orm-query --features libsql -E 'binary(libsql_mutations_test)'
cargo nextest run -p toolu-orm-query --features rusqlite -E 'binary(rusqlite_mutations_test)'
docker compose -f docker-compose.test.yaml up -d --wait
TEST_DB_PORT=5434 cargo nextest run -p toolu-orm-core -p toolu-orm-macros -p toolu-orm-query -p toolu-orm-connection -p toolu-orm-cli --features postgres -E 'binary(postgres_mutations_test)'
```

## Tests

| Lane | Binary | Test |
|---|---|---|
| libsql-only | libsql_mutations_test | or_replace_replaces_conflicting_row |
| libsql-only | libsql_mutations_test | or_ignore_keeps_original_row |
| rusqlite-only | rusqlite_mutations_test | or_replace_replaces_conflicting_row |
| rusqlite-only | rusqlite_mutations_test | or_ignore_keeps_original_row |
| postgres | postgres_mutations_test | insert_update_delete_round_trip |
| postgres | postgres_mutations_test | or_replace_updates_the_conflicting_row |
| postgres | postgres_mutations_test | or_ignore_keeps_the_existing_row |
