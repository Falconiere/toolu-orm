# Expression fragments

**Feature:** `Expr::to_sql_fragment_for(start, dialect)` renders a WHERE fragment whose placeholders continue from `start`, so builders can chain filters, joins, and raw SQL without renumbering. SQLite gets `?N`, Postgres gets `$N`.
**Drivers:** both dialects. The numbering itself is pure SQL generation (no database); the fragment-relative `?N` rule of issue #131 is additionally executed against real rows on libsql, rusqlite and Postgres. The same operators are executed against real rows in [Filters](filters.md).
**Spec:** AC-11, issues #113 and #131.

## What is proven

| Expression (start = 3) | SQLite | Postgres | Params |
|---|---|---|---|
| `eq / ne / gt / lt / gte / lte / like` | `"users"."age" > ?3` | `"users"."age" > $3` | 1 |
| `in_list([a, b])` | `"users"."id" IN (?3, ?4)` | `... IN ($3, $4)` | 2 |
| `not_in([a, b])` | `... NOT IN (?3, ?4)` | `... NOT IN ($3, $4)` | 2 |
| `between(25, 30)` | `... BETWEEN ?3 AND ?4` | `... BETWEEN $3 AND $4` | 2 |
| `is_null / is_not_null` | `... IS NULL` | same | 0 |
| `a.and(b).or(c)` | `(("a" = ?3 AND "b" = ?4) OR "c" = ?5)` | `$3 $4 $5` | 3, in order |
| `in_list([])` | `1 = 0` | `1 = 0` | 0 |
| `not_in([])` | `1 = 1` | `1 = 1` | 0 |
| `cmp.and(Expr::raw("... ?", [v]))` | `"repo" = ?1 AND ... ?2` | `"repo" = $1 AND ... $2` | 2, in order |
| `cmp.and(Expr::raw("x = ?1 OR y = ?1", [v]))` | `"repo" = ?1 AND x = ?2 OR y = ?2` | `... x = $2 OR y = $2` | 2 — the fragment binds one |

The empty-list rows pin a fix made by this program: the renderer used to emit `IN ()`, which Postgres rejects with SQLSTATE 42601 (SQLite silently treated it as false). Drizzle renders the same constants.

The `Expr::raw` row pins the fix for issue #113: a raw fragment nested inside `and`/`or` used to number its bare `?` placeholders from the caller's `start` alone, colliding with placeholders already emitted by earlier siblings in the same tree, instead of continuing from `start + <params already emitted>`.

### A numbered `?N` is fragment-relative (issue #131)

#113 fixed the bare `?`. The numbered form kept its literal index, so a fragment
was correct only when it happened to render first:

```rust
Expr::raw("x = ?1", vec![v]).to_sql_fragment_for(3, Dialect::Sqlite);
// before: ("x = ?1", [v])   the value landed at index 3 — the predicate compared another clause's value
// after:  ("x = ?3", [v])
```

A fragment is a frame. `base` is the index its first value takes, and every
number in its text — written or implied — is shifted by `base - 1`:

| Written | Means | At base 4 |
|---|---|---|
| `?N`, `N >= 1` | the fragment's own `N`-th value | `?N` → `?{N+3}` |
| the same `N` twice | one parameter read twice, bound once | `?4 … ?4` |
| a bare `?` | one more than the largest number assigned so far — SQLite's own rule for anonymous parameters | `"?1 … ?"` → `?4 … ?5` |
| `?0` | nothing: indices are 1-based | left verbatim, so the engine refuses to prepare |
| a digit run no `usize` can hold or shift | nothing | left verbatim |

The values a fragment binds never changed: it still binds exactly what it was
given, once, in order, whatever the text references. So a fragment that names
more placeholders than it supplies values is an authoring bug in either form —
the index shifts with the frame and then addresses a later clause's value, or
none — and `SharedBind` / `Scalar::shared` remain the only way to name a value
another predicate bound.

**Breaking change.** A fragment written against 0.8.x that relied on `?N` being
an absolute statement index now renders a different index. Replace it with the
bare form (`?`), or with a handle when the intent was cross-predicate reuse.

Executed proof lives in `{libsql,rusqlite,postgres}_raw_fragment_test`: a
`SELECT` whose raw fragment is not first returns the rows that match the value
the fragment bound. Before the fix the same query returned **zero** rows on
libsql (it compared `src_id` against the `rel` value), and Postgres rejected the
bind outright — *bind message supplies 2 parameters, but prepared statement
requires 1*.

### The typed-column surface

`query_column_test` pins the other half: which `Column<T>` gets which operation trait, and what each one renders. The table above proves the operators; this proves the typed handle reaches them at all.

| Surface | Proven |
|---|---|
| `Column::new` / `qualified()` | stores `table` and `name`; renders `"orders"."total"` |
| `CommonOps` | `eq` / `ne` / `in_list` / `not_in` / `is_null` / `is_not_null` |
| `NumericOps` | `gt` / `lt` / `gte` / `lte` / `between` on `Integer`, `Real`, `BigInt`, `SmallInt`, `Date`, `Time` — and, as compile-time assertions, on `Timestamp` |
| `TextOps` | `like` on `Text`, `Varchar<N>`, `Uuid`, `Date`, `Time` |
| `Expr::and` / `or` | parentheses, and left-to-right parameter numbering |
| `start` offsets | a plain fragment and a nested `and` both number from `start`, not from 1 |
| `Expr::json_extract` | `json_extract("records"."data", '$.key')` carrying `eq` and `like` |
| `Expr::raw` | bare `?` renumbered from the buffer's position; `?N` renumbered from it too, as the fragment's own `N`-th value |
| `Column::asc` / `desc` | an `OrderBy` rendering `ASC` / `DESC` |
| `Column::equals` | a column-to-column `JoinCondition` that binds nothing |

This binary compiles in the default lane (orm-core with libsql) and again in the postgres lane (orm-core with postgres + libsql), where `Dialect::CURRENT` is Postgres. Its placeholder assertions therefore name `Dialect::Sqlite` explicitly; the two that bind nothing (`is_null` / `is_not_null`) stay on the dialect-implicit `to_sql_fragment`, which keeps that entry point covered in both lanes. Dialect parity for the operators themselves is the `expr_offset_and_nesting_test` table above.

The suite existed from the start but was never compiled: its entry file was `mod.rs`, which Cargo's integration-test auto-discovery does not look at, and `orm-core` declares no `[[test]]` targets — so all 33 tests were inert (issue #125). `scripts/check-test-targets.sh` now fails on any test file no cargo target builds.

## How to run

```sh
cargo nextest run -p toolu-orm-core -E 'binary(expr_offset_and_nesting_test) | binary(expr_test) | binary(expr_raw_bind_index_test) | binary(expr_raw_numbered_index_test) | binary(query_column_test)'
cargo nextest run -p toolu-orm-query --features libsql -E 'binary(libsql_raw_fragment_test)'
cargo nextest run -p toolu-orm-query --features rusqlite,sqlite-vec -E 'binary(rusqlite_raw_fragment_test)'
TEST_DB_PORT=5434 cargo nextest run -p toolu-orm-query --features postgres -E 'binary(postgres_raw_fragment_test)'
```

## Tests

| Lane | Binary | Test |
|---|---|---|
| default | expr_offset_and_nesting_test | sqlite_all_comparison_operators_at_offset_three |
| default | expr_offset_and_nesting_test | postgres_all_comparison_operators_at_offset_three |
| default | expr_offset_and_nesting_test | sqlite_in_list_not_in_and_between_at_offset_three |
| default | expr_offset_and_nesting_test | postgres_in_list_not_in_and_between_at_offset_three |
| default | expr_offset_and_nesting_test | is_null_produces_no_params_regardless_of_offset_or_dialect |
| default | expr_offset_and_nesting_test | nested_and_or_preserves_parentheses_sqlite |
| default | expr_offset_and_nesting_test | nested_and_or_preserves_parentheses_postgres |
| default | expr_offset_and_nesting_test | empty_in_list_sqlite_renders_contradiction |
| default | expr_offset_and_nesting_test | empty_in_list_postgres_renders_contradiction |
| default | expr_offset_and_nesting_test | empty_not_in_sqlite_renders_tautology |
| default | expr_offset_and_nesting_test | empty_not_in_postgres_renders_tautology |
| default | expr_raw_bind_index_test | comparison_and_raw_sqlite_numbers_sequentially |
| default | expr_raw_bind_index_test | comparison_and_raw_postgres_numbers_sequentially |
| default | expr_raw_bind_index_test | raw_and_comparison_sqlite_numbers_sequentially |
| default | expr_raw_bind_index_test | raw_or_raw_sqlite_numbers_sequentially |
| default | expr_raw_bind_index_test | raw_or_raw_postgres_numbers_sequentially |
| default | expr_raw_bind_index_test | three_raw_siblings_chained_number_sequentially |
| default | expr_raw_bind_index_test | raw_nested_two_levels_in_and_or |
| default | expr_raw_bind_index_test | non_default_start_offsets_compose_with_prior_params |
| default | expr_raw_bind_index_test | zero_param_raw_sibling_does_not_reserve_an_index |
| default | expr_raw_numbered_index_test | a_bare_only_fragment_renders_exactly_as_it_did_before |
| default | expr_raw_numbered_index_test | a_bare_placeholder_takes_one_more_than_the_largest_number_assigned |
| default | expr_raw_numbered_index_test | a_digit_run_too_large_for_an_index_stays_verbatim_and_never_panics |
| default | expr_raw_numbered_index_test | a_number_after_a_bare_placeholder_can_name_it |
| default | expr_raw_numbered_index_test | a_number_may_address_an_earlier_value_out_of_reading_order |
| default | expr_raw_numbered_index_test | a_numbered_fragment_after_a_typed_predicate_compares_its_own_value |
| default | expr_raw_numbered_index_test | a_numbered_placeholder_at_base_one_is_unchanged |
| default | expr_raw_numbered_index_test | a_numbered_placeholder_takes_the_index_its_value_actually_lands_on |
| default | expr_raw_numbered_index_test | a_scalar_fragment_is_numbered_from_its_own_base |
| default | expr_raw_numbered_index_test | a_scalar_fragment_reuses_one_value_across_two_occurrences |
| default | expr_raw_numbered_index_test | an_index_past_the_supplied_values_is_shifted_like_any_other |
| default | expr_raw_numbered_index_test | an_index_whose_shift_would_overflow_stays_verbatim |
| default | expr_raw_numbered_index_test | index_zero_is_not_a_placeholder_and_stays_verbatim |
| default | expr_raw_numbered_index_test | one_number_used_twice_renders_one_index_and_binds_one_value |
| default | expr_raw_numbered_index_test | postgres_renders_the_same_frame_with_dollar_placeholders |
| default | expr_raw_numbered_index_test | the_mixed_fragment_shifts_whole_at_an_offset |
| default | expr_raw_numbered_index_test | two_numbers_address_the_two_values_in_order |
| default | query_column_test | common_ops::column_new_stores_table_and_name |
| default | query_column_test | common_ops::column_qualified_format |
| default | query_column_test | common_ops::text_column_eq_produces_correct_sql |
| default | query_column_test | common_ops::column_ne_produces_correct_sql |
| default | query_column_test | common_ops::column_in_list_produces_correct_sql |
| default | query_column_test | common_ops::column_not_in_produces_correct_sql |
| default | query_column_test | common_ops::column_is_null_produces_no_params |
| default | query_column_test | common_ops::column_is_not_null_produces_correct_sql |
| default | query_column_test | common_ops::expr_and_produces_correct_sql_with_param_numbering |
| default | query_column_test | common_ops::expr_or_produces_correct_sql_with_param_numbering |
| default | query_column_test | numeric_ops::integer_column_gt_produces_correct_sql |
| default | query_column_test | numeric_ops::integer_column_lt_produces_correct_sql |
| default | query_column_test | numeric_ops::integer_column_gte_produces_correct_sql |
| default | query_column_test | numeric_ops::integer_column_lte_produces_correct_sql |
| default | query_column_test | numeric_ops::column_between_produces_correct_sql |
| default | query_column_test | numeric_ops::real_column_gt_produces_correct_sql |
| default | query_column_test | numeric_ops::bigint_column_has_numeric_ops |
| default | query_column_test | numeric_ops::smallint_column_has_numeric_ops |
| default | query_column_test | numeric_ops::to_sql_fragment_respects_start_offset |
| default | query_column_test | numeric_ops::nested_and_respects_start_offset |
| default | query_column_test | text_and_special_ops::text_column_like_produces_correct_sql |
| default | query_column_test | text_and_special_ops::varchar_column_like_produces_correct_sql |
| default | query_column_test | text_and_special_ops::uuid_column_like_produces_correct_sql |
| default | query_column_test | text_and_special_ops::date_column_like_produces_correct_sql |
| default | query_column_test | text_and_special_ops::date_column_gt_produces_correct_sql |
| default | query_column_test | text_and_special_ops::time_column_has_text_and_numeric_ops |
| default | query_column_test | text_and_special_ops::column_asc_produces_correct_sql |
| default | query_column_test | text_and_special_ops::column_desc_produces_correct_sql |
| default | query_column_test | text_and_special_ops::column_equals_produces_join_condition_sql |
| default | query_column_test | text_and_special_ops::expr_json_extract_eq_produces_correct_sql |
| default | query_column_test | text_and_special_ops::expr_json_extract_like_produces_correct_sql |
| default | query_column_test | text_and_special_ops::expr_raw_replaces_bare_question_marks_with_numbered_params |
| default | query_column_test | text_and_special_ops::expr_raw_numbers_an_already_numbered_param_from_the_offset |
| libsql-only | libsql_raw_fragment_test | a_reused_number_in_a_later_fragment_reads_its_own_value |
| libsql-only | libsql_raw_fragment_test | the_statement_that_ran_names_the_fragments_own_index_twice |
| libsql-only | libsql_raw_fragment_test | two_numbers_out_of_reading_order_bound_the_right_way_round |
| rusqlite-only | rusqlite_raw_fragment_test | a_bare_placeholder_after_a_numbered_one_takes_the_next_value |
| rusqlite-only | rusqlite_raw_fragment_test | a_reused_number_in_a_later_fragment_reads_its_own_value |
| rusqlite-only | rusqlite_raw_fragment_test | two_numbers_out_of_reading_order_bound_the_right_way_round |
| postgres | postgres_raw_fragment_test | a_bare_placeholder_after_a_numbered_one_takes_the_next_value |
| postgres | postgres_raw_fragment_test | a_reused_number_in_a_later_fragment_reads_its_own_value |
| postgres | postgres_raw_fragment_test | two_numbers_out_of_reading_order_bound_the_right_way_round |
