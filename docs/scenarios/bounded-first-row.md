# Bounded first-row fetch

**Feature:** `fetch_one` and `fetch_optional` send a query bounded to at most one row (`SelectBuilder::to_first_row_sql`), so the database stops producing rows the caller will never see and at most one row is ever decoded.
**Drivers:** libsql, rusqlite, Postgres.
**Issue:** [#87](https://github.com/Falconiere/toolu-orm/issues/87).

## What is proven

Both methods used to run the unbounded query through `fetch_all`, decode every
matching row and drop all but the first. On a table of 100,000 rows a one-row
result cost 100,000 decodes, and a row that could not decode anywhere in the
result set failed an otherwise valid first row.

Each driver seeds a real `item` table with ids `1..=10_000` and decodes through
`#[derive(FromRow)]` structs whose `id` field carries a `#[from_row(with = …)]`
hook that counts every decode.

| Call | Table state | Result |
|---|---|---|
| `fetch_one::<CountedId>` ordered by `id` | 10,000 rows | `id = 1`, decode counter `1` |
| `fetch_optional::<CountedId>` ordered by `id` | 10,000 rows | `Some(1)`, decode counter `1` |
| either, with a decoder that rejects every `id != 1` | 10,000 rows | the row with `id = 1`, not a `RowMapping` error |
| either, filtered to `id > 10_000` | 10,000 rows | `NotFound { table: "item" }` / `None`, decode counter `0` |
| either, with an explicit `.limit(0)` | 10,000 rows | `NotFound` / `None`, decode counter `0` |
| `fetch_one` with `ORDER BY id DESC` and `.offset(2)` | 10,000 rows | `id = 9_998`, decode counter `1` |
| `fetch_one` with `ORDER BY id ASC` and `.limit(5)` | 10,000 rows | `id = 1`, decode counter `1` |
| `fetch_one` at 1, 10, 1,000 and 10,000 rows | each in turn | `id = 1`, decode counter `1` every time |

The rendering is pinned without a database in `first_row_sql_test`: no limit
becomes `LIMIT 1`, a positive limit is clamped to `1`, an explicit `LIMIT 0`
stays `0`, and a negative limit is passed through unchanged — SQLite reads it as
"no limit" and Postgres rejects it, so clamping it would change an observable
result on one driver or the other. Filters, `ORDER BY` and `OFFSET` are
preserved with their parameter numbering in both dialects, and `to_sql_for`
still renders the builder's own limit.

The decode counter is a process-global `AtomicUsize` reset before each fetch;
`cargo nextest` runs every test in its own process, which is what makes an exact
assertion on it sound.

## How to run

```sh
cargo nextest run -p toolu-orm-query -E 'binary(first_row_sql_test)'
cargo nextest run -p toolu-orm-query --features libsql -E 'binary(libsql_first_row_test)'
cargo nextest run -p toolu-orm-query --features rusqlite -E 'binary(rusqlite_first_row_test)'
docker compose -f docker-compose.test.yaml up -d --wait
TEST_DB_PORT=5434 cargo nextest run -p toolu-orm -p toolu-orm-core -p toolu-orm-macros -p toolu-orm-query -p toolu-orm-connection -p toolu-orm-cli -p toolu-orm-facade-consumer --features postgres -E 'binary(postgres_first_row_test)'
```

## Tests

| Lane | Binary | Test |
|---|---|---|
| default | first_row_sql_test | no_limit_becomes_limit_one |
| default | first_row_sql_test | positive_limit_is_clamped_to_one |
| default | first_row_sql_test | explicit_limit_zero_is_preserved |
| default | first_row_sql_test | negative_limit_is_passed_through_unchanged |
| default | first_row_sql_test | filters_order_and_offset_are_preserved_with_continued_parameters |
| default | first_row_sql_test | postgres_renders_dollar_placeholders |
| default | first_row_sql_test | to_sql_for_still_renders_the_builders_own_limit |
| default | first_row_sql_test | raw_builder_still_renders_a_bound |
| libsql-only | libsql_first_row_test | fetch_one_decodes_one_row_for_a_large_match |
| libsql-only | libsql_first_row_test | fetch_optional_decodes_one_row_for_a_large_match |
| libsql-only | libsql_first_row_test | first_row_survives_a_later_row_that_cannot_decode |
| libsql-only | libsql_first_row_test | no_matching_row_is_not_found_and_none |
| libsql-only | libsql_first_row_test | explicit_limit_zero_yields_no_row |
| libsql-only | libsql_first_row_test | filters_order_offset_and_positive_limit_still_select_the_first_row |
| libsql-only | libsql_first_row_test | decoded_rows_stay_one_as_cardinality_grows |
| rusqlite-only | rusqlite_first_row_test | fetch_one_decodes_one_row_for_a_large_match |
| rusqlite-only | rusqlite_first_row_test | fetch_optional_decodes_one_row_for_a_large_match |
| rusqlite-only | rusqlite_first_row_test | first_row_survives_a_later_row_that_cannot_decode |
| rusqlite-only | rusqlite_first_row_test | no_matching_row_is_not_found_and_none |
| rusqlite-only | rusqlite_first_row_test | explicit_limit_zero_yields_no_row |
| rusqlite-only | rusqlite_first_row_test | filters_order_offset_and_positive_limit_still_select_the_first_row |
| rusqlite-only | rusqlite_first_row_test | decoded_rows_stay_one_as_cardinality_grows |
| postgres | postgres_first_row_test | fetch_one_decodes_one_row_for_a_large_match |
| postgres | postgres_first_row_test | fetch_optional_decodes_one_row_for_a_large_match |
| postgres | postgres_first_row_test | first_row_survives_a_later_row_that_cannot_decode |
| postgres | postgres_first_row_test | no_matching_row_is_not_found_and_none |
| postgres | postgres_first_row_test | explicit_limit_zero_yields_no_row |
| postgres | postgres_first_row_test | filters_order_offset_and_positive_limit_still_select_the_first_row |
| postgres | postgres_first_row_test | decoded_rows_stay_one_as_cardinality_grows |
