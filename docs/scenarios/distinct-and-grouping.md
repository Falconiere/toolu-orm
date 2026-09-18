# DISTINCT, GROUP BY, HAVING and aggregate projections

**Feature:** `SelectBuilder` gains `distinct()`, `group_by()` / `group_by_scalar()` and `having()`, and `Scalar` gains the aggregate constructors `count_star`, `count`, `count_distinct`, `sum`, `max`, `min` and `avg`. Aggregates are ordinary `Scalar` nodes, so they drop into `column_scalar`, into `order_by` through `asc`/`desc`, into arithmetic through the `std::ops` operators, and into the comparisons that build a `HAVING` predicate. Grouping, deduplication and pagination all stay database-side.
**Drivers:** rusqlite, libsql and Postgres execute the scenarios; `grouping_sql_test` and `scalar_expr_test::aggregates` render them per dialect.
**Spec:** issue #109.

## What is proven

Seed on every driver — `source_files(id, source_id, path, status, size_bytes)` plus `sources(id, label)`:

| id | source_id | path | status | size_bytes |
|---|---|---|---|---|
| f1 | s1 | `src/a.rs` | indexed | 10 |
| f2 | s1 | `src/a.rs` | indexed | 20 |
| f3 | s1 | `src/b.rs` | pending | 30 |
| f4 | s1 | `src/c.rs` | failed | 40 |
| f5 | s2 | `src/a.rs` | indexed | 50 |
| f6 | s1 | `src/b.rs` | indexed | 60 |
| f7 | s1 | `src/c.rs` | failed | 5 |

`sources` holds `s1 → primary` and `s2 → secondary`. The rows are chosen so every claim has a *discriminating* answer: `s1` spans six rows over three distinct paths, `status` splits 4 / 2 / 1, `(source_id, status)` makes four groups, and `s1`'s sizes total 165 over six rows — a mean of 27.5, which integer division could not produce.

| Built from | SQL | Rows / result |
|---|---|---|
| `.columns_qualified(&[&PATH]).distinct().order_by(PATH.asc())` | `SELECT DISTINCT "source_files"."path" …` | `src/a.rs src/b.rs src/c.rs` |
| the same, `.limit(2).offset(1)` | `… LIMIT ?2 OFFSET ?3` | `src/b.rs src/c.rs` — **not** the `src/a.rs src/b.rs` the same page without `DISTINCT` returns |
| `.column_scalar(Scalar::count_star(), "n").group_by(&STATUS)` | `SELECT "status", COUNT(*) AS "n" … GROUP BY "source_files"."status"` | `indexed 4`, `failed 2`, `pending 1` |
| `.group_by(&SOURCE_ID).group_by(&STATUS)` | `GROUP BY …"source_id", …"status"` | four rows, one per occurring combination |
| `count_star` / `count_distinct` / `sum` / `max` / `min` / `avg` over `s1` | `COUNT(*)`, `COUNT(DISTINCT "…"."path")`, `SUM(…)`, … | `6, 3, 165, 60, 5, 27.5` |
| the same over `s2`, a single-row group | — | `1, 1, 50, 50, 50, 50.0` |
| `.order_by(OrderBy::alias_desc("n"))` | `ORDER BY "n" DESC` | `indexed, failed, pending`; `alias_asc` reverses it |
| `.having(Scalar::count_star().gt(Scalar::bind(1)))` | `… WHERE …= ?1 … HAVING COUNT(*) > ?2` | groups larger than one; the `HAVING` bind is numbered **after** the `WHERE` bind |
| two `having` calls | `HAVING … AND …` | both conjuncts apply |
| `HAVING COUNT(*) > 100` | — | `[]`, `count() == 0`, `exists() == false` |
| an aggregate with **no** `GROUP BY` over zero matching rows | `SELECT SUM(…), COUNT(*) … WHERE … = ?1` | exactly **one** row: `SUM` `NULL`, `COUNT(*)` `0` |
| `group_by(&s.column(&LABEL))` after a join | `GROUP BY "s"."label"` | `primary 6`, `secondary 1` |
| `column_expr(raw, alias)` beside `COUNT(*)`, `group_by_scalar(Scalar::sql(…))` | unchanged | the raw escape hatches still compose |

### Grouped-count semantics

`count()` means **how many rows the unpaginated query returns**. That already described the ungrouped case; with `GROUP BY` the returned rows are *groups*, so the count is the number of groups, and `HAVING` participates because it removes groups.

Appending `GROUP BY` to a bare `SELECT COUNT(*)` would return one row per group, and reading the first of them would report the size of whichever group came first. So a grouped — or `DISTINCT` — builder counts a derived table instead:

```sql
SELECT COUNT(*) FROM (SELECT "status", COUNT(*) AS "n" FROM "source_files" GROUP BY "source_files"."status") AS "toolu_count"
```

The inner statement keeps the select list (it is what `DISTINCT` deduplicates on, and it may bind), every join, `WHERE`, `GROUP BY` and `HAVING`; only `ORDER BY` and pagination are dropped, exactly as they always were. The derived table is always aliased, because Postgres requires it and SQLite accepts it.

On the seed above, `count()` over `GROUP BY status` is **3** — asserted against both traps: not 7 (the raw rows) and not 4 (the largest group). `count()` over the distinct-path listing is **3**, not 6. The wrap applies **only** when `distinct || !group_bys.is_empty()`, so every existing builder renders byte-identical count SQL.

`exists()` needs no wrap: `SELECT 1 … GROUP BY x HAVING …` yields one row per surviving group, so `EXISTS` is true iff a group survives. `DISTINCT` is deliberately not rendered there — deduplicating `SELECT 1` cannot change whether a row exists.

### Portability notes

- **`DISTINCT` and `ORDER BY` must agree.** Postgres enforces *for `SELECT DISTINCT`, `ORDER BY` expressions must appear in select list*; SQLite does not. Order by a projected term: `columns_qualified` pairs with a typed column's `asc()`/`desc()`, which both render `"table"."column"`. The Postgres suite executes that pairing.
- **Grouped select lists.** Postgres requires every non-aggregated select-list item to appear in `GROUP BY`; SQLite silently returns an arbitrary row per group. The builder renders what it is told and does not police this, exactly as `Scalar::func` does not translate function names between dialects.
- **`SUM` and `AVG` on Postgres return `numeric`** for a `bigint` argument, which the current row decoders do not map to a Rust scalar. `COUNT(*)`, `COUNT(DISTINCT …)`, `MAX` and `MIN` over a `bigint` stay `bigint`. So the Postgres suite asserts those four, and `SUM`/`AVG` are proven live on both SQLite drivers and rendered for both dialects.
- **Integer division is the engine's.** `Scalar::sum(x) / Scalar::count_star()` renders `(SUM(x) / COUNT(*))`, and SQLite divides two integers as integers — 165 / 6 is 27, not 27.5, which is what `AVG` is for. libsql's decoder will not coerce that INTEGER into an `f64`, so the Rust field type has to match what the engine returned.
- **`HAVING` should repeat the aggregate** rather than naming a projection's alias: SQLite resolves the alias, Postgres does not.

## How to run

```sh
cargo nextest run -p toolu-orm-core -E 'binary(scalar_expr_test)'
cargo nextest run -p toolu-orm-query -E 'binary(grouping_sql_test)'
cargo nextest run -p toolu-orm-facade-consumer -E 'binary(facade_only_grouping_test)'
cargo nextest run -p toolu-orm-query --features rusqlite,sqlite-vec -E 'binary(rusqlite_grouping_test)'
cargo nextest run -p toolu-orm-query --features libsql -E 'binary(libsql_grouping_test)'
docker compose -f docker-compose.test.yaml up -d --wait
TEST_DB_PORT=5434 cargo nextest run -p toolu-orm-query --features postgres -E 'binary(postgres_grouping_test)'
```

## Tests

| Lane | Binary | Test |
|---|---|---|
| default | scalar_expr_test | aggregates::a_plain_count_argument_carries_no_distinct_keyword |
| default | scalar_expr_test | aggregates::aggregates_compose_with_arithmetic_and_comparisons |
| default | scalar_expr_test | aggregates::an_aggregate_can_name_a_column_of_an_aliased_relation |
| default | scalar_expr_test | aggregates::an_aggregate_over_a_bound_value_numbers_from_the_offset |
| default | scalar_expr_test | aggregates::count_distinct_puts_the_keyword_inside_the_parentheses |
| default | scalar_expr_test | aggregates::count_star_renders_the_star_and_binds_nothing |
| default | scalar_expr_test | aggregates::every_aggregate_renders_its_own_uppercase_name |
| default | scalar_expr_test | aggregates::the_same_aggregate_renders_dollar_placeholders_on_postgres |
| default | grouping_sql_test | count_tests::a_distinct_count_wraps_so_it_counts_distinct_rows |
| default | grouping_sql_test | count_tests::a_grouped_count_wraps_the_statement_in_an_aliased_derived_table |
| default | grouping_sql_test | count_tests::a_having_clause_participates_in_the_count |
| default | grouping_sql_test | count_tests::an_ungrouped_count_renders_the_plain_form_unchanged |
| default | grouping_sql_test | count_tests::an_ungrouped_exists_renders_the_plain_form_unchanged |
| default | grouping_sql_test | count_tests::exists_carries_the_grouping_but_not_distinct |
| default | grouping_sql_test | count_tests::the_counted_inner_statement_drops_ordering_and_pagination |
| default | grouping_sql_test | count_tests::the_derived_table_is_aliased_on_postgres_too |
| default | grouping_sql_test | render_tests::a_grouped_report_renders_the_issues_statement |
| default | grouping_sql_test | render_tests::a_qualified_grouping_key_names_the_joined_alias |
| default | grouping_sql_test | render_tests::distinct_is_idempotent |
| default | grouping_sql_test | render_tests::distinct_renders_before_the_select_list_and_after_it_the_page |
| default | grouping_sql_test | render_tests::every_aggregate_projection_renders_in_one_select_list |
| default | grouping_sql_test | render_tests::every_binding_clause_numbers_in_render_order |
| default | grouping_sql_test | render_tests::having_is_numbered_after_the_where_clause |
| default | grouping_sql_test | render_tests::several_grouping_terms_render_in_call_order |
| default | grouping_sql_test | render_tests::several_having_conjuncts_join_with_and |
| default | grouping_sql_test | render_tests::the_raw_escape_hatch_still_projects_and_groups |
| default | grouping_sql_test | render_tests::the_same_statement_numbers_with_dollar_placeholders_on_postgres |
| default | facade_only_grouping_test | a_distinct_listing_composes_through_the_facade |
| default | facade_only_grouping_test | a_grouped_count_wraps_a_derived_table_through_the_facade |
| default | facade_only_grouping_test | a_grouped_report_composes_through_the_facade |
| rusqlite-only | rusqlite_grouping_test | aggregates::aggregates_compose_with_arithmetic_in_a_projection |
| rusqlite-only | rusqlite_grouping_test | aggregates::count_distinct_differs_from_count_star_where_duplicates_exist |
| rusqlite-only | rusqlite_grouping_test | aggregates::every_aggregate_is_computed_per_group |
| rusqlite-only | rusqlite_grouping_test | counting::a_count_over_no_surviving_group_is_zero |
| rusqlite-only | rusqlite_grouping_test | counting::a_distinct_count_reports_distinct_rows_not_underlying_rows |
| rusqlite-only | rusqlite_grouping_test | counting::a_grouped_count_reports_the_number_of_groups |
| rusqlite-only | rusqlite_grouping_test | counting::an_ungrouped_count_still_counts_rows |
| rusqlite-only | rusqlite_grouping_test | counting::exists_follows_the_having_clause |
| rusqlite-only | rusqlite_grouping_test | counting::exists_is_false_when_no_group_survives_the_filter |
| rusqlite-only | rusqlite_grouping_test | counting::having_narrows_the_grouped_count |
| rusqlite-only | rusqlite_grouping_test | counting::pagination_does_not_change_the_count |
| rusqlite-only | rusqlite_grouping_test | distinct::a_page_of_a_distinct_listing_is_a_page_of_distinct_rows |
| rusqlite-only | rusqlite_grouping_test | distinct::an_offset_past_the_last_distinct_row_returns_no_rows |
| rusqlite-only | rusqlite_grouping_test | distinct::distinct_collapses_the_duplicate_paths |
| rusqlite-only | rusqlite_grouping_test | distinct::the_undeduplicated_listing_still_holds_every_duplicate |
| rusqlite-only | rusqlite_grouping_test | grouped_reports::a_grouped_report_paginates_by_group |
| rusqlite-only | rusqlite_grouping_test | grouped_reports::grouping_by_one_column_returns_one_row_per_status |
| rusqlite-only | rusqlite_grouping_test | grouped_reports::grouping_by_two_columns_returns_one_row_per_combination |
| rusqlite-only | rusqlite_grouping_test | grouped_reports::ordering_by_an_aggregate_alias_ranks_the_groups |
| rusqlite-only | rusqlite_grouping_test | grouped_reports::the_ascending_alias_order_reverses_it |
| rusqlite-only | rusqlite_grouping_test | having::a_having_that_excludes_every_group_returns_no_rows |
| rusqlite-only | rusqlite_grouping_test | having::a_where_bind_and_a_having_bind_keep_their_own_values |
| rusqlite-only | rusqlite_grouping_test | having::an_ungrouped_aggregate_over_no_rows_returns_one_null_row |
| rusqlite-only | rusqlite_grouping_test | having::grouping_an_empty_filter_result_returns_no_rows |
| rusqlite-only | rusqlite_grouping_test | having::having_filters_groups_by_their_aggregate |
| rusqlite-only | rusqlite_grouping_test | having::several_having_conjuncts_all_apply |
| rusqlite-only | rusqlite_grouping_test | having::the_having_bound_is_a_parameter_not_a_literal |
| rusqlite-only | rusqlite_grouping_test | qualified::a_raw_grouping_term_groups_by_a_computed_key |
| rusqlite-only | rusqlite_grouping_test | qualified::an_aggregate_can_read_a_column_of_the_aliased_relation |
| rusqlite-only | rusqlite_grouping_test | qualified::grouping_by_a_joined_aliased_column_counts_per_label |
| rusqlite-only | rusqlite_grouping_test | qualified::the_raw_column_expr_escape_hatch_still_projects |
| libsql-only | libsql_grouping_test | aggregates::aggregates_compose_with_arithmetic_in_a_projection |
| libsql-only | libsql_grouping_test | aggregates::count_distinct_differs_from_count_star_where_duplicates_exist |
| libsql-only | libsql_grouping_test | aggregates::every_aggregate_is_computed_per_group |
| libsql-only | libsql_grouping_test | counting::a_count_over_no_surviving_group_is_zero |
| libsql-only | libsql_grouping_test | counting::a_distinct_count_reports_distinct_rows_not_underlying_rows |
| libsql-only | libsql_grouping_test | counting::a_grouped_count_reports_the_number_of_groups |
| libsql-only | libsql_grouping_test | counting::an_ungrouped_count_still_counts_rows |
| libsql-only | libsql_grouping_test | counting::exists_follows_the_having_clause |
| libsql-only | libsql_grouping_test | counting::exists_is_false_when_no_group_survives_the_filter |
| libsql-only | libsql_grouping_test | counting::having_narrows_the_grouped_count |
| libsql-only | libsql_grouping_test | counting::pagination_does_not_change_the_count |
| libsql-only | libsql_grouping_test | distinct::a_page_of_a_distinct_listing_is_a_page_of_distinct_rows |
| libsql-only | libsql_grouping_test | distinct::an_offset_past_the_last_distinct_row_returns_no_rows |
| libsql-only | libsql_grouping_test | distinct::distinct_collapses_the_duplicate_paths |
| libsql-only | libsql_grouping_test | distinct::the_undeduplicated_listing_still_holds_every_duplicate |
| libsql-only | libsql_grouping_test | grouped_reports::a_grouped_report_paginates_by_group |
| libsql-only | libsql_grouping_test | grouped_reports::grouping_by_one_column_returns_one_row_per_status |
| libsql-only | libsql_grouping_test | grouped_reports::grouping_by_two_columns_returns_one_row_per_combination |
| libsql-only | libsql_grouping_test | grouped_reports::ordering_by_an_aggregate_alias_ranks_the_groups |
| libsql-only | libsql_grouping_test | grouped_reports::the_ascending_alias_order_reverses_it |
| libsql-only | libsql_grouping_test | having::a_having_that_excludes_every_group_returns_no_rows |
| libsql-only | libsql_grouping_test | having::a_where_bind_and_a_having_bind_keep_their_own_values |
| libsql-only | libsql_grouping_test | having::an_ungrouped_aggregate_over_no_rows_returns_one_null_row |
| libsql-only | libsql_grouping_test | having::grouping_an_empty_filter_result_returns_no_rows |
| libsql-only | libsql_grouping_test | having::having_filters_groups_by_their_aggregate |
| libsql-only | libsql_grouping_test | having::several_having_conjuncts_all_apply |
| libsql-only | libsql_grouping_test | having::the_having_bound_is_a_parameter_not_a_literal |
| libsql-only | libsql_grouping_test | qualified::a_raw_grouping_term_groups_by_a_computed_key |
| libsql-only | libsql_grouping_test | qualified::an_aggregate_can_read_a_column_of_the_aliased_relation |
| libsql-only | libsql_grouping_test | qualified::grouping_by_a_joined_aliased_column_counts_per_label |
| libsql-only | libsql_grouping_test | qualified::the_raw_column_expr_escape_hatch_still_projects |
| postgres | postgres_grouping_test | aggregates::the_bigint_aggregates_are_computed_per_group |
| postgres | postgres_grouping_test | counting::a_distinct_count_reports_distinct_rows_not_underlying_rows |
| postgres | postgres_grouping_test | counting::a_grouped_count_reports_the_number_of_groups |
| postgres | postgres_grouping_test | counting::an_ungrouped_count_still_counts_rows |
| postgres | postgres_grouping_test | counting::exists_follows_the_having_clause |
| postgres | postgres_grouping_test | counting::having_narrows_the_grouped_count |
| postgres | postgres_grouping_test | distinct::a_page_of_a_distinct_listing_is_a_page_of_distinct_rows |
| postgres | postgres_grouping_test | distinct::distinct_collapses_the_duplicate_paths |
| postgres | postgres_grouping_test | grouped_reports::grouping_by_one_column_returns_one_row_per_status |
| postgres | postgres_grouping_test | grouped_reports::grouping_by_two_columns_returns_one_row_per_combination |
| postgres | postgres_grouping_test | grouped_reports::ordering_by_an_aggregate_alias_ranks_the_groups |
| postgres | postgres_grouping_test | having::a_having_that_excludes_every_group_returns_no_rows |
| postgres | postgres_grouping_test | having::a_where_bind_and_a_having_bind_keep_their_own_values |
| postgres | postgres_grouping_test | having::an_ungrouped_aggregate_over_no_rows_returns_one_null_row |
| postgres | postgres_grouping_test | having::having_filters_groups_by_their_aggregate |
| postgres | postgres_grouping_test | having::several_having_conjuncts_all_apply |
| postgres | postgres_grouping_test | qualified::grouping_by_a_joined_aliased_column_counts_per_label |
