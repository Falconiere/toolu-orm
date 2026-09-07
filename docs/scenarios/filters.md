# Filters

**Feature:** typed `Column<T>` constants build `Expr` filters: `eq` / `ne` / `in_list` / `not_in` / `is_null` / `is_not_null` on every column, `like` on text, `gt` / `lt` / `gte` / `lte` / `between` on numbers, combined with `.and()` / `.or()`. `SelectBuilder::filter` can be called repeatedly; filters are ANDed and their parameters keep numbering across calls.
**Drivers:** libsql, rusqlite, Postgres. The fragments themselves are pinned per dialect in [Expression fragments](expr-fragments.md).
**Spec:** AC-11.

## What is proven

Seed on every driver: `u1 Ann 30 a@x.io`, `u2 Bea 25 b@x.io`, `u3 Cid NULL c@y.io`, `u4 Dee 41 d@y.io`.

| Filter | Expected ids |
|---|---|
| `NAME.eq("Bea")` / `NAME.ne("Bea")` | `u2` / `u1 u3 u4` |
| `ID.in_list([u1, u4])` / `ID.not_in([u1, u4])` | `u1 u4` / `u2 u3` |
| `AGE.is_null()` / `AGE.is_not_null()` | `u3` / `u1 u2 u4` |
| `EMAIL.like("%@y.io")` | `u3 u4` |
| `AGE.gt(30)` / `AGE.lt(30)` / `AGE.gte(30)` / `AGE.lte(30)` | `u4` / `u2` / `u1 u4` / `u1 u2` |
| `AGE.between(25, 30)` | `u1 u2` |
| `.filter(AGE.gte(25)).filter(EMAIL.like("%x.io")).filter(ID.in_list([u1, u2, u3]))` | `u1 u2` (params 1..5 across three filters) |
| `NAME.eq("Ann").and(AGE.gt(20)).or(NAME.eq("Dee"))` | `u1 u4` (parentheses keep precedence) |
| `ID.in_list([])` | executes, matches nothing (renders `1 = 0`) |

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
| libsql-only | libsql_reads_test | eq_and_ne_match_expected_rows |
| libsql-only | libsql_reads_test | in_list_and_not_in_match_expected_rows |
| libsql-only | libsql_reads_test | is_null_and_like_match_expected_rows |
| libsql-only | libsql_reads_test | gt_and_lt_match_expected_rows |
| libsql-only | libsql_reads_test | gte_and_lte_match_expected_rows |
| libsql-only | libsql_reads_test | between_matches_inclusive_range |
| libsql-only | libsql_reads_test | chained_filters_continue_param_offset_into_in_list |
| libsql-only | libsql_reads_test | and_or_combination_matches_expected_rows |
| libsql-only | libsql_reads_test | empty_in_list_executes_without_driver_error |
| rusqlite-only | rusqlite_reads_test | eq_and_ne_match_expected_rows |
| rusqlite-only | rusqlite_reads_test | in_list_and_not_in_match_expected_rows |
| rusqlite-only | rusqlite_reads_test | is_null_and_like_match_expected_rows |
| rusqlite-only | rusqlite_reads_test | gt_and_lt_match_expected_rows |
| rusqlite-only | rusqlite_reads_test | gte_and_lte_match_expected_rows |
| rusqlite-only | rusqlite_reads_test | between_matches_inclusive_range |
| rusqlite-only | rusqlite_reads_test | chained_filters_continue_param_offset_into_in_list |
| rusqlite-only | rusqlite_reads_test | and_or_combination_matches_expected_rows |
| rusqlite-only | rusqlite_reads_test | empty_in_list_executes_without_driver_error |
| rusqlite-only | rusqlite_reads_test | empty_not_in_matches_every_row |
| postgres | postgres_reads_test | every_filter_operator_selects_the_expected_rows |
| postgres | postgres_reads_test | three_filters_keep_numbering_params_past_the_first_two |
| postgres | postgres_reads_test | nested_and_or_keeps_precedence |
| postgres | postgres_reads_test | empty_in_list_executes_and_matches_nothing |
