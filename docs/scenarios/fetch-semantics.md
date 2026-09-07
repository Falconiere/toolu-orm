# Fetch semantics

**Feature:** `SelectBuilder` adds `fetch_all`, `fetch_one`, `fetch_optional`, `count`, and `exists` on top of `.execute()`. They are async on libsql and Postgres, sync on rusqlite, with the same results.
**Drivers:** libsql, rusqlite, Postgres.
**Spec:** AC-12.

## What is proven

| Call | Table state | Result |
|---|---|---|
| `fetch_one::<User>` | empty | `Err(QueryError::NotFound { table: "users" })` |
| `fetch_optional::<User>` | empty, then one row | `None`, then `Some(row)` |
| `count` | three rows | `3` |
| `exists` with a matching filter / a non-matching filter | three rows | `true` / `false` |
| `fetch_one` with two matching rows and `order_by(ID.desc())` | two rows | the first row of the ordered result (`u2`) |

The last row pins current behavior: `fetch_one` does not error on more than one match (spec Q4).

## How to run

```sh
cargo nextest run -p toolu-orm-query --features libsql -E 'binary(libsql_reads_test)'
cargo nextest run -p toolu-orm-query --features rusqlite -E 'binary(rusqlite_reads_test)'
docker compose -f docker-compose.test.yaml up -d --wait
TEST_DB_PORT=5434 cargo nextest run -p toolu-orm-core -p toolu-orm-macros -p toolu-orm-query -p toolu-orm-connection -p toolu-orm-cli --features postgres -E 'binary(postgres_reads_test)'
```

## Tests

| Lane | Binary | Test |
|---|---|---|
| libsql-only | libsql_reads_test | fetch_one_on_empty_table_returns_not_found |
| libsql-only | libsql_reads_test | fetch_optional_returns_none_then_some |
| libsql-only | libsql_reads_test | count_returns_three_after_three_inserts |
| libsql-only | libsql_reads_test | exists_true_for_matching_filter_false_otherwise |
| libsql-only | libsql_reads_test | fetch_one_with_two_matches_returns_first_by_order |
| rusqlite-only | rusqlite_reads_test | fetch_one_on_empty_table_returns_not_found |
| rusqlite-only | rusqlite_reads_test | fetch_optional_returns_none_then_some |
| rusqlite-only | rusqlite_reads_test | count_returns_three_after_three_inserts |
| rusqlite-only | rusqlite_reads_test | exists_true_for_matching_filter_false_otherwise |
| rusqlite-only | rusqlite_reads_test | fetch_one_with_two_matches_returns_first_by_order |
| postgres | postgres_reads_test | fetch_one_on_empty_table_is_not_found |
| postgres | postgres_reads_test | fetch_optional_none_then_some |
| postgres | postgres_reads_test | count_and_exists_reflect_real_rows |
| postgres | postgres_reads_test | fetch_one_with_two_matches_returns_the_first_by_order |
