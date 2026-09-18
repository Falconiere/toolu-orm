# Scalar expressions and LIKE ESCAPE

**Feature:** `Scalar` is the value half of the expression tree — a column, a bound value, a validated function call, arithmetic, `||` concatenation, or a `CASE`. It composes into `Expr` comparisons and drops into `SelectBuilder::column_scalar`, `order_by`, `InsertBuilder::set_scalar` and `UpdateBuilder::set_scalar`. `TextOps::like_escape` (and `Scalar::like_escape`) add an explicit escape character, bound as a parameter, so `%` and `_` typed by a user match literally.
**Drivers:** libsql, rusqlite, Postgres — plus dialect-only rendering in `scalar_expr_test` / `scalar_expr_sql_test`. `datetime(...)` is SQLite's, so the mixed-precision timestamp rows are SQLite-only; everything else is proven on all three.
**Spec:** issue #112.

## What is proven

Seed on every driver — `memories(id, body, created_at, last_accessed, access_count)`:

| id | body | created_at | last_accessed | access_count |
|---|---|---|---|---|
| m1 | `100% cotton` | `2026-09-18T10:00:00Z` | NULL | 0 |
| m2 | `100 percent cotton` | `2026-09-18T10:00:00.123Z` | `2026-09-19T08:00:00Z` | 5 |
| m3 | `a_b` | `2026-09-18T09:59:59.999999Z` | NULL | 12 |
| m4 | `axb` | `2026-09-17T10:00:00Z` | `2026-09-16T08:00:00Z` | 0 |

| Built from | SQL | Rows / result |
|---|---|---|
| `BODY.like_escape(format!("%{}%", like_pattern_literal("100%", '\\')), '\\')` | `"body" LIKE ?1 ESCAPE ?2` | `m1` only |
| `BODY.like("%100%%")` (no escape) | `"body" LIKE ?1` | `m1 m2` — the wildcard reading |
| `BODY.like_escape(like_pattern_literal("a_b", '\\'), '\\')` | `… ESCAPE ?2` | `m3` only (`m4` is `axb`) |
| `Scalar::func("datetime", [col])?.gte(Scalar::func("datetime", [bind])?)` | `datetime("created_at") >= datetime(?1)` | `m1 m2` (SQLite) |
| `Scalar::col(&CREATED_AT).gte(Scalar::bind(cutoff))` | `"created_at" >= ?1` | `m1` — text order drops the `.123` row |
| `Scalar::func("coalesce", [last_accessed, created_at])?` as a projection | `coalesce(…) AS "effective"` | `created_at` for the NULL rows |
| the same term in `order_by(…desc())` | `ORDER BY coalesce(…) DESC` | `m2 m1 m3 m4` |
| `set_scalar(&HITS, Scalar::col(&HITS) + Scalar::bind(1))` | `"access_count" = ("access_count" + ?1)` | only the filtered row, +1 then +2 |
| `Scalar::case_when(…).otherwise(…)` / `.end()` | `CASE WHEN … THEN … ELSE … END` / no `ELSE` | branch value / `NULL` |
| `Scalar::col(&ID).concat(bind).concat(substr(…))` | `(("id" \|\| ?1) \|\| substr(…))` | `m1:100` |
| `Scalar::func("drop table users; --", …)` | — | `DbCoreError::InvalidScalarFunction`, no SQL built |

Parameter numbering is the point of the design: every node takes its index from `start + params.len()`, so one statement can bind in its projection, its `SET`, its `WHERE` and its `ORDER BY` and still number `?1..?n` in the order the values are returned. `count()` / `exists()` do not render the projection, so a bound projection contributes no parameter there and the filter still numbers from 1.

The escape character is bound rather than interpolated (`… ESCAPE ?2` / `$5`), which both engines accept and which leaves no route for a quote into the SQL; a Rust `char` is one character by construction, which is what they require of that operand.

Postgres resolves function overloads by exact type and a bound `Value::Integer` arrives as `bigint`, so `substr`'s positions are literals in the Postgres suite (`substr(text, int, int)` has no `bigint` overload) where the SQLite suites bind them.

## How to run

```sh
cargo nextest run -p toolu-orm-core -E 'binary(scalar_expr_test)'
cargo nextest run -p toolu-orm-query -E 'binary(scalar_expr_sql_test)'
cargo nextest run -p toolu-orm-query --features rusqlite,sqlite-vec -E 'binary(rusqlite_scalar_expr_test)'
cargo nextest run -p toolu-orm-query --features libsql -E 'binary(libsql_scalar_expr_test)'
docker compose -f docker-compose.test.yaml up -d --wait
TEST_DB_PORT=5434 cargo nextest run -p toolu-orm-query --features postgres -E 'binary(postgres_scalar_expr_test)'
```

## Tests

| Lane | Binary | Test |
|---|---|---|
| default | scalar_expr_test | arithmetic::a_column_plus_one_renders_parenthesized_with_one_bind |
| default | scalar_expr_test | arithmetic::a_raw_operand_continues_numbering_after_its_left_sibling |
| default | scalar_expr_test | arithmetic::every_operator_renders_its_symbol |
| default | scalar_expr_test | arithmetic::nesting_keeps_parentheses_and_orders_binds_left_to_right |
| default | scalar_expr_test | case_terms::a_case_without_else_ends_after_its_last_branch |
| default | scalar_expr_test | case_terms::a_column_order_term_still_renders_exactly_as_before |
| default | scalar_expr_test | case_terms::a_scalar_order_term_renders_its_direction_and_binds |
| default | scalar_expr_test | case_terms::a_two_branch_case_with_else_numbers_predicate_then_value |
| default | scalar_expr_test | comparisons::a_call_on_each_side_binds_only_the_right_hand_value |
| default | scalar_expr_test | comparisons::a_literal_pattern_escapes_every_wildcard_and_the_escape_itself |
| default | scalar_expr_test | comparisons::every_comparison_operator_renders_between_two_scalars |
| default | scalar_expr_test | comparisons::like_escape_binds_the_pattern_then_the_escape_character |
| default | scalar_expr_test | comparisons::like_escape_continues_numbering_after_an_earlier_conjunct |
| default | scalar_expr_test | comparisons::like_without_an_escape_renders_the_same_as_the_column_operator |
| default | scalar_expr_test | functions::a_call_over_a_column_renders_on_both_dialects |
| default | scalar_expr_test | functions::a_call_without_arguments_renders_empty_parentheses |
| default | scalar_expr_test | functions::a_name_that_is_not_a_plain_identifier_is_refused |
| default | scalar_expr_test | functions::an_underscore_leading_name_is_accepted |
| default | scalar_expr_test | functions::nested_arguments_number_left_to_right_from_the_offset |
| default | scalar_expr_test | leaves::a_bound_leaf_takes_the_offset_it_is_rendered_at |
| default | scalar_expr_test | leaves::a_column_leaf_renders_qualified_and_binds_nothing |
| default | scalar_expr_test | leaves::a_raw_leaf_numbers_its_placeholders_from_the_offset |
| default | scalar_expr_test | leaves::a_sql_leaf_passes_its_text_through_unchanged |
| default | scalar_expr_sql_test | an_order_term_that_binds_is_numbered_after_the_filter |
| default | scalar_expr_sql_test | column_expr_and_column_scalar_render_in_call_order |
| default | scalar_expr_sql_test | count_drops_a_bound_projection_and_numbers_the_filter_from_one |
| default | scalar_expr_sql_test | insert_mixes_bound_values_with_a_computed_one |
| default | scalar_expr_sql_test | select_numbers_projection_then_filter_then_order_on_sqlite |
| default | scalar_expr_sql_test | select_numbers_the_same_statement_with_dollar_placeholders_on_postgres |
| default | scalar_expr_sql_test | set_expr_still_renders_raw_text_without_a_parameter |
| default | scalar_expr_sql_test | update_keeps_the_same_order_on_postgres |
| default | scalar_expr_sql_test | update_numbers_set_scalars_before_the_filter |
| rusqlite-only | rusqlite_scalar_expr_test | arithmetic::an_arithmetic_self_update_raises_only_the_filtered_row |
| rusqlite-only | rusqlite_scalar_expr_test | arithmetic::binds_land_in_select_set_and_where_in_statement_order |
| rusqlite-only | rusqlite_scalar_expr_test | nulls::coalesce_falls_back_to_created_at_when_last_accessed_is_null |
| rusqlite-only | rusqlite_scalar_expr_test | nulls::ordering_by_coalesce_ranks_rows_by_their_effective_time |
| rusqlite-only | rusqlite_scalar_expr_test | projections::a_case_without_else_yields_null_when_no_branch_matches |
| rusqlite-only | rusqlite_scalar_expr_test | projections::case_and_concatenation_return_their_branch_values |
| rusqlite-only | rusqlite_scalar_expr_test | timestamps::a_plain_string_comparison_drops_the_sub_second_row |
| rusqlite-only | rusqlite_scalar_expr_test | timestamps::datetime_normalizes_mixed_precision_timestamps |
| rusqlite-only | rusqlite_scalar_expr_test | wildcards::like_escape_matches_only_the_literal_percent_row |
| rusqlite-only | rusqlite_scalar_expr_test | wildcards::like_escape_matches_only_the_literal_underscore_row |
| rusqlite-only | rusqlite_scalar_expr_test | wildcards::the_same_search_without_an_escape_matches_both_percent_rows |
| libsql-only | libsql_scalar_expr_test | arithmetic::an_arithmetic_self_update_raises_only_the_filtered_row |
| libsql-only | libsql_scalar_expr_test | arithmetic::binds_land_in_select_set_and_where_in_statement_order |
| libsql-only | libsql_scalar_expr_test | nulls::coalesce_falls_back_to_created_at_when_last_accessed_is_null |
| libsql-only | libsql_scalar_expr_test | nulls::ordering_by_coalesce_ranks_rows_by_their_effective_time |
| libsql-only | libsql_scalar_expr_test | projections::a_case_without_else_yields_null_when_no_branch_matches |
| libsql-only | libsql_scalar_expr_test | projections::case_and_concatenation_return_their_branch_values |
| libsql-only | libsql_scalar_expr_test | timestamps::a_plain_string_comparison_drops_the_sub_second_row |
| libsql-only | libsql_scalar_expr_test | timestamps::datetime_normalizes_mixed_precision_timestamps |
| libsql-only | libsql_scalar_expr_test | wildcards::like_escape_matches_only_the_literal_percent_row |
| libsql-only | libsql_scalar_expr_test | wildcards::like_escape_matches_only_the_literal_underscore_row |
| libsql-only | libsql_scalar_expr_test | wildcards::the_same_search_without_an_escape_matches_both_percent_rows |
| postgres | postgres_scalar_expr_test | arithmetic::an_arithmetic_self_update_raises_only_the_filtered_row |
| postgres | postgres_scalar_expr_test | arithmetic::binds_land_in_select_set_and_where_in_statement_order |
| postgres | postgres_scalar_expr_test | nulls::coalesce_falls_back_to_created_at_when_last_accessed_is_null |
| postgres | postgres_scalar_expr_test | nulls::ordering_by_coalesce_ranks_rows_by_their_effective_time |
| postgres | postgres_scalar_expr_test | projections::a_case_without_else_yields_null_when_no_branch_matches |
| postgres | postgres_scalar_expr_test | projections::case_and_concatenation_return_their_branch_values |
| postgres | postgres_scalar_expr_test | wildcards::a_bound_scalar_comparison_selects_the_row_at_the_cutoff |
| postgres | postgres_scalar_expr_test | wildcards::like_escape_matches_only_the_literal_percent_row |
| postgres | postgres_scalar_expr_test | wildcards::like_escape_matches_only_the_literal_underscore_row |
| postgres | postgres_scalar_expr_test | wildcards::the_same_search_without_an_escape_matches_both_percent_rows |
