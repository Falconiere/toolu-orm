# FromRow derive

**Feature:** `#[derive(FromRow)]` maps a row into a struct positionally (`row.try_get(idx)` in field order) and exposes `REQUIRED_COLUMNS`, so `select_for::<T>()` selects exactly those columns in that order.
**Drivers:** Postgres decodes for real. On libsql the derive's `from_libsql_row` is a documented error stub (`crates/orm-macros/src/from_row_expand.rs`): "`<Type> is only decoded from Postgres rows`". Single-driver suites implement `from_row` by hand (see [Lanes](lanes.md)).
**Spec:** AC-18.

## What is proven

- A `Person { id: String, age: Option<i64> }` decodes two real Postgres rows, one with `age = NULL`, into `None` and `Some(30)`.
- Selecting fewer columns than the struct has (`SELECT id`) fails with `DbError::RowMapping("... column 1 (age): ...")` instead of a panic.
- The same derived struct, queried through `LibsqlConnection::query_map` on an in-memory libsql database, returns `DbError::RowMapping` ending in "is only decoded from Postgres rows". This pins the limitation so a future fix (spec Q3) has to update this scenario.
- The derive output itself (impl generated for a named struct, `REQUIRED_COLUMNS` order) is checked by the orm-macros suite.

## How to run

```sh
docker compose -f docker-compose.test.yaml up -d --wait
TEST_DB_PORT=5434 cargo nextest run -p toolu-orm-core -p toolu-orm-macros -p toolu-orm-query -p toolu-orm-connection -p toolu-orm-cli --features postgres -E 'binary(from_row_derive_live_test) | binary(from_row_test)'
```

## Tests

| Lane | Binary | Test |
|---|---|---|
| postgres | from_row_derive_live_test | derive_decodes_postgres_rows_with_null_as_none |
| postgres | from_row_derive_live_test | derive_fewer_columns_than_required_is_row_mapping |
| postgres | from_row_derive_live_test | derive_on_libsql_row_returns_documented_stub_error |
| postgres | from_row_test | test_from_row_generates_impl_for_named_struct |
| postgres | from_row_test | from_row_generates_required_columns |
