# SQLite offset without limit

**Feature:** `.offset(n)` without a paired `.limit(...)` renders `LIMIT -1 OFFSET ...` for SQLite (`SelectBuilder::to_sql` / `to_sql_for`), instead of a standalone `OFFSET` SQLite rejects as a syntax error. Postgres keeps its already-valid standalone `OFFSET`.
**Drivers:** libsql, rusqlite, Postgres.
**Issue:** [#92](https://github.com/Falconiere/toolu-orm/issues/92).

## What is proven

Before this fix, `SelectBuilder::new("item").offset(n).to_sql()` rendered a
bare `OFFSET ?N` on SQLite with no `LIMIT`, which `sqlite3_prepare` (and
libsql, which shares SQLite's grammar) rejects at prepare time. `fetch_all`
on such a builder failed every time an offset was set without a limit.
Issue #87 (`to_first_row_sql_for`, `fetch_one`/`fetch_optional`) already
supplies its own bound limit and was never affected; the hole was in the
unbounded rendering path (`to_sql`, `to_sql_for`, and therefore `fetch_all`).

The fix emits a literal `LIMIT -1` ahead of `OFFSET` for SQLite whenever no
limit is set — SQLite reads a negative `LIMIT` as "no limit". The literal
contributes no bound parameter, so placeholder numbering for `OFFSET` and any
later clause is unchanged. Postgres's standalone `OFFSET` is untouched.

Each driver seeds a real `item` table with ids `1..=10`, ordered by `id`, and
asserts the exact returned id set for each offset:

| Call | Table state | Result |
|---|---|---|
| `.offset(0)` | 10 rows | all 10 rows, ids `1..=10` |
| `.offset(3)` | 10 rows | ids `4..=10` |
| `.offset(10)` (== row count) | 10 rows | empty — a valid boundary, not an error |
| `.offset(1_000)` (beyond row count) | 10 rows | empty — proves execution, not a syntax error |
| `.filter(id > 2).offset(1)` | 10 rows | ids `4..=10` — WHERE parameter numbering stays correct into `OFFSET` |
| `.limit(3).offset(2)` | 10 rows | ids `[3, 4, 5]` — the pre-existing limit+offset path stays unaffected |

The rendering is pinned without a database in `offset_without_limit_sql_test`:
SQLite offset-only renders `LIMIT -1 OFFSET ?N`, offset `0` renders the same
shape, Postgres offset-only keeps a bare `OFFSET $N`, an explicit
`.limit().offset()` on SQLite is unaffected, and a WHERE filter ahead of
offset-only pagination keeps correct placeholder numbering on both dialects.

## How to run

```sh
cargo nextest run -p toolu-orm-query -E 'binary(offset_without_limit_sql_test)'
cargo nextest run -p toolu-orm-query --features libsql -E 'binary(libsql_offset_without_limit_test)'
cargo nextest run -p toolu-orm-query --features rusqlite -E 'binary(rusqlite_offset_without_limit_test)'
docker compose -f docker-compose.test.yaml up -d --wait
TEST_DB_PORT=5434 cargo nextest run -p toolu-orm -p toolu-orm-core -p toolu-orm-macros -p toolu-orm-query -p toolu-orm-connection -p toolu-orm-cli -p toolu-orm-facade-consumer --features postgres -E 'binary(postgres_offset_without_limit_test)'
```

## Tests

| Lane | Binary | Test |
|---|---|---|
| default | offset_without_limit_sql_test | sqlite_offset_only_renders_limit_negative_one |
| default | offset_without_limit_sql_test | sqlite_offset_zero_still_renders_limit_negative_one |
| default | offset_without_limit_sql_test | postgres_offset_only_keeps_standalone_offset |
| default | offset_without_limit_sql_test | sqlite_explicit_limit_and_offset_unaffected |
| default | offset_without_limit_sql_test | sqlite_where_filter_then_offset_only_keeps_parameter_numbering |
| default | offset_without_limit_sql_test | postgres_where_filter_then_offset_only_keeps_parameter_numbering |
| libsql-only | libsql_offset_without_limit_test | offset_zero_returns_every_row |
| libsql-only | libsql_offset_without_limit_test | positive_offset_skips_the_leading_rows |
| libsql-only | libsql_offset_without_limit_test | offset_equal_to_row_count_returns_empty |
| libsql-only | libsql_offset_without_limit_test | offset_beyond_row_count_returns_empty |
| libsql-only | libsql_offset_without_limit_test | where_filter_then_offset_only_selects_the_right_rows |
| libsql-only | libsql_offset_without_limit_test | explicit_limit_and_offset_still_page_correctly |
| rusqlite-only | rusqlite_offset_without_limit_test | offset_zero_returns_every_row |
| rusqlite-only | rusqlite_offset_without_limit_test | positive_offset_skips_the_leading_rows |
| rusqlite-only | rusqlite_offset_without_limit_test | offset_equal_to_row_count_returns_empty |
| rusqlite-only | rusqlite_offset_without_limit_test | offset_beyond_row_count_returns_empty |
| rusqlite-only | rusqlite_offset_without_limit_test | where_filter_then_offset_only_selects_the_right_rows |
| rusqlite-only | rusqlite_offset_without_limit_test | explicit_limit_and_offset_still_page_correctly |
| postgres | postgres_offset_without_limit_test | offset_zero_returns_every_row |
| postgres | postgres_offset_without_limit_test | positive_offset_skips_the_leading_rows |
| postgres | postgres_offset_without_limit_test | offset_equal_to_row_count_returns_empty |
| postgres | postgres_offset_without_limit_test | offset_beyond_row_count_returns_empty |
| postgres | postgres_offset_without_limit_test | where_filter_then_offset_only_selects_the_right_rows |
| postgres | postgres_offset_without_limit_test | explicit_limit_and_offset_still_page_correctly |
