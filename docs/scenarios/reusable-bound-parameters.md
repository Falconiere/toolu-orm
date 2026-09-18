# Reusable bound parameters

**Feature:** `SharedBind` and `SharedBindList` bind a value — or a list — **once** and let every predicate built from the handle reference the same placeholder. `query_column::SharedOps` adds `eq_shared` / `ne_shared` / `in_shared` / `not_in_shared` to every column, `Scalar::shared` puts a handle in value position, and `expr::BoundParams` is the statement-wide buffer that records what each handle bound.
**Drivers:** rusqlite, libsql and Postgres execute the scenarios; `reusable_bind_sql_test`, orm-core's `shared_bind_test` / `bound_params_test` render them per dialect; `facade_only_shared_bind_test` proves the surface composes with `toolu-orm` as a consumer's only dependency.
**Spec:** issue #116.

## The problem this solves

A two-direction graph lookup names the same inputs twice:

```rust
src_id.eq(fid).and(dst_id.in_list(&files))
  .or(dst_id.eq(fid).and(src_id.in_list(&files)))
```

Every occurrence allocated its own placeholder, so the statement bound `2 * file_count + 5` values. With **16,381** working-set paths that is **32,767** — one past SQLite's default `SQLITE_MAX_VARIABLE_NUMBER` of 32,766 — and preparation fails with *variable number must be between ?1 and ?32766*.

With handles, each input is bound once and referenced from both orientations:

```rust
let candidate = SharedBind::new("file:r:candidate.rs");
let files = SharedBindList::new(working_set);      // 16,381 paths

Edges::select()
  .column_expr(r#"CAST(COALESCE(SUM("edges"."weight"), 0) AS BIGINT)"#, "weight")
  .filter(edges::rel.eq("co_changed"))
  .filter(edges::src_kind.eq("file"))
  .filter(edges::dst_kind.eq("file"))
  .filter(
    edges::src_id.eq_shared(&candidate).and(edges::dst_id.in_shared(&files))
      .or(edges::dst_id.eq_shared(&candidate).and(edges::src_id.in_shared(&files))),
  )
```

```sql
SELECT CAST(COALESCE(SUM("edges"."weight"), 0) AS BIGINT) AS "weight" FROM "edges"
 WHERE "edges"."rel" = ?1 AND "edges"."src_kind" = ?2 AND "edges"."dst_kind" = ?3
   AND (("edges"."src_id" = ?4 AND "edges"."dst_id" IN (?5, …, ?16385))
     OR ("edges"."dst_id" = ?4 AND "edges"."src_id" IN (?5, …, ?16385)))
```

| Form | Parameters | Outcome on SQLite |
|---|---|---|
| `eq` / `in_list` | **32,767** | `prepare` refused: *variable number must be between ?1 and ?32766* |
| `eq_shared` / `in_shared` | **16,385** | prepares, executes, returns the correct weight |

16,385 is 16,381 files + one candidate + the three fixed predicates, each still bound separately because they are ordinary values.

## What is proven

The seed is deliberately discriminating. `edges` holds four rows, so a query that ignored one orientation, one relation or the working-set membership would give a different number:

| `rel` | `src_id` | `dst_id` | `weight` | Why it is there |
|---|---|---|---|---|
| `co_changed` | `file:r:candidate.rs` | `file:r:7.rs` | 7 | the outgoing direction |
| `co_changed` | `file:r:11.rs` | `file:r:candidate.rs` | 11 | the incoming direction |
| `imports` | `file:r:candidate.rs` | `file:r:7.rs` | 100 | right pair, wrong relation |
| `co_changed` | `file:r:candidate.rs` | `file:r:outside.rs` | 50 | right relation, outside the working set |

Expected weight: **18**. One orientation alone gives 7 or 11; dropping the `rel` predicate adds 100; dropping the working-set predicate adds 50. All three drivers return 18.

### Identity is the handle, never the value

| Case | Behaviour |
|---|---|
| One handle used N times | one parameter; every occurrence renders the same index |
| Two handles over **equal** values | two parameters — an explicit requirement of the issue |
| A cloned handle | the same binding as its original |
| A handle used once | byte-identical to the `eq` / `in_list` form |
| A handle never used | binds nothing; a handle is not a registration |
| An empty `SharedBindList` | `1 = 0` (`in_shared`) / `1 = 1` (`not_in_shared`), zero parameters, identical at every occurrence |

A handle carries its own value, so there is no unbound state and no statement it "belongs" to: using one in a statement that has never seen it simply binds it there. Two statements built from one handle each bind it once, and rendering a builder twice is byte-identical — the ledger belongs to the render, never to the handle or the builder.

### Numbering across every boundary

Position lives in `BoundParams`: each node takes `next_index()` as it writes, and `BoundParams::nested` is the single place that re-bases the count for a statement spliced after other clauses. A shared handle is the one node that may not push — it hands back the index it already took, leaving the live length, and therefore every later node's numbering, untouched.

| Boundary | Proven by |
|---|---|
| Nested `AND` / `OR` | one index at all four occurrences, both dialects |
| Two separate `.filter()` calls | `one_handle_spans_projection_two_filters_having_and_order_by` |
| Projection, `GROUP BY`/`HAVING`, `ORDER BY` | the same test, five clause positions on one index |
| `JOIN … ON` | `a_join_predicate_shares_with_the_where_clause` |
| CTE body ↔ outer `WHERE` | the handle takes the *low* index, because `WITH` renders first |
| `UNION` arms | both arms read one placeholder |
| Subquery, first use **inside** | the outer clause reuses the index the subquery allocated |
| Subquery, first use **outside** | the subquery reuses the outer index |
| Derived-table count wrap | numbers from `?1` again and shares inside itself |
| `Expr::raw` / `Scalar::raw` | a raw `?` after a reuse takes the next *unused* index; a literal `?N` still passes through verbatim and cannot address a handle |
| `UPDATE` `SET` ↔ `WHERE`, `DELETE`, `INSERT … VALUES`, `INSERT … SELECT` ↔ `ON CONFLICT` | one parameter each, with the rows read back |
| A `SelectSource` implemented outside this workspace | it may not override the defaulted `to_select_sql_into`, so its statement renders with its own ledger and a handle used on both sides binds **twice** — one extra parameter, correct SQL, and never a wrong index |

Every SQL assertion names its dialect through `to_sql_for(Dialect::…)`; `to_sql()` follows `Dialect::CURRENT`, which differs between the default and postgres lanes. Postgres repeats `$N` exactly as SQLite repeats `?N`.

`shared_bind_test::nesting::every_rendered_index_is_backed_by_a_bound_value` and `reusable_bind_sql_test::nesting` additionally check *programmatically* that the distinct placeholder indices are exactly `1..=params.len()` — a wrong shared index would either reference a value that was never bound or leave one unreferenced.

## Tests

| Lane | Binary | Test |
|---|---|---|
| default | bound_params_test | a_default_buffer_is_an_empty_one |
| default | bound_params_test | a_handle_moves_across_a_thread_and_still_renders_one_placeholder |
| default | bound_params_test | an_empty_buffer_numbers_the_first_placeholder_one |
| default | bound_params_test | extend_appends_a_rendered_fragments_values_in_order |
| default | bound_params_test | handles_and_expressions_stay_send_and_sync |
| default | bound_params_test | handles_built_on_many_threads_all_stay_independent |
| default | bound_params_test | one_handle_shared_across_threads_is_still_one_binding |
| default | bound_params_test | nested_at_one_is_the_identity_frame |
| default | bound_params_test | nested_carries_the_binding_ledger_out_of_the_frame |
| default | bound_params_test | nested_continues_a_live_buffer_from_its_next_index |
| default | bound_params_test | nested_on_an_empty_buffer_is_the_standalone_fragment_frame |
| default | bound_params_test | push_returns_the_index_it_took_and_advances_the_count |
| default | facade_only_shared_bind_test | a_reused_binding_takes_one_placeholder_through_the_facade |
| default | facade_only_shared_bind_test | the_same_predicate_renders_dollar_placeholders_through_the_facade |
| default | facade_only_shared_bind_test | two_handles_over_equal_values_stay_independent_through_the_facade |
| default | reusable_bind_sql_test | clauses::a_delete_shares_a_handle_across_nested_and_or |
| default | reusable_bind_sql_test | clauses::a_join_predicate_shares_with_the_where_clause |
| default | reusable_bind_sql_test | clauses::an_insert_select_shares_a_handle_with_its_conflict_assignment |
| default | reusable_bind_sql_test | clauses::an_insert_values_row_shares_a_handle_between_two_columns |
| default | reusable_bind_sql_test | clauses::an_update_shares_a_handle_between_its_set_clause_and_its_where |
| default | reusable_bind_sql_test | clauses::one_handle_spans_projection_two_filters_having_and_order_by |
| default | reusable_bind_sql_test | clauses::the_same_statement_shares_the_handle_with_dollar_placeholders |
| default | reusable_bind_sql_test | issue_case::both_orientations_name_the_same_placeholders_on_sqlite |
| default | reusable_bind_sql_test | issue_case::rendering_the_same_builder_twice_gives_the_identical_statement |
| default | reusable_bind_sql_test | issue_case::the_owned_form_still_binds_every_input_twice |
| default | reusable_bind_sql_test | issue_case::the_same_handles_bind_once_again_in_a_second_statement |
| default | reusable_bind_sql_test | issue_case::the_same_query_renders_dollar_placeholders_on_postgres |
| default | reusable_bind_sql_test | issue_case::the_shared_form_binds_each_input_once |
| default | reusable_bind_sql_test | issue_case::the_shared_parameters_are_the_fixed_predicates_then_the_candidate_then_the_files |
| default | reusable_bind_sql_test | nesting::a_handle_first_used_in_a_cte_body_is_reused_by_the_outer_where |
| default | reusable_bind_sql_test | nesting::a_handle_first_used_inside_a_subquery_is_reused_by_a_later_clause |
| default | reusable_bind_sql_test | nesting::a_handle_first_used_outside_is_reused_inside_a_subquery |
| default | reusable_bind_sql_test | nesting::a_scalar_subquery_projection_shares_with_the_where_that_follows_it |
| default | reusable_bind_sql_test | nesting::the_derived_table_count_wrap_numbers_from_one_and_shares_inside_itself |
| default | reusable_bind_sql_test | nesting::union_arms_share_one_placeholder_with_the_arm_before_them |
| default | shared_bind_test | foreign_source::a_foreign_source_still_numbers_from_where_its_siblings_left_off |
| default | shared_bind_test | foreign_source::a_handle_inside_a_foreign_source_binds_again_rather_than_taking_a_wrong_index |
| default | shared_bind_test | identity::a_cloned_handle_is_the_same_binding |
| default | shared_bind_test | identity::a_handle_exposes_the_payload_it_binds |
| default | shared_bind_test | identity::a_handle_that_is_never_used_binds_nothing |
| default | shared_bind_test | identity::a_handle_used_once_binds_exactly_what_the_owned_form_binds |
| default | shared_bind_test | identity::an_empty_list_handle_renders_the_constant_at_every_occurrence |
| default | shared_bind_test | identity::rendering_the_same_expression_twice_is_idempotent |
| default | shared_bind_test | identity::the_same_handle_renders_in_scalar_and_predicate_position |
| default | shared_bind_test | identity::two_handles_holding_equal_values_stay_independent |
| default | shared_bind_test | identity::two_list_handles_holding_equal_values_stay_independent |
| default | shared_bind_test | nesting::a_fragment_spliced_at_an_offset_numbers_its_shared_index_from_there |
| default | shared_bind_test | nesting::a_handle_reused_across_nested_and_or_takes_one_placeholder_on_sqlite |
| default | shared_bind_test | nesting::a_negated_shared_predicate_renders_the_same_indices |
| default | shared_bind_test | nesting::an_owned_predicate_beside_a_shared_one_keeps_binding_per_occurrence |
| default | shared_bind_test | nesting::every_rendered_index_is_backed_by_a_bound_value |
| default | shared_bind_test | nesting::the_same_predicate_renders_dollar_placeholders_on_postgres |
| default | shared_bind_test | raw_and_offsets::a_between_beside_a_reuse_keeps_its_own_two_placeholders |
| default | shared_bind_test | raw_and_offsets::a_literal_index_in_a_raw_fragment_cannot_address_a_handle |
| default | shared_bind_test | raw_and_offsets::a_raw_fragment_after_a_reuse_takes_the_next_unused_index |
| default | shared_bind_test | raw_and_offsets::a_raw_fragment_before_a_reuse_still_owns_the_low_indices |
| default | shared_bind_test | raw_and_offsets::a_shared_list_and_an_owned_list_of_the_same_values_stay_apart |
| default | shared_bind_test | raw_and_offsets::a_shared_scalar_composes_with_arithmetic_and_binds_once |
| default | shared_bind_test | raw_and_offsets::a_shared_scalar_inside_a_case_shares_with_the_predicate_around_it |
| postgres | postgres_reusable_bind_test | clauses::a_cte_body_and_the_outer_where_share_one_placeholder |
| postgres | postgres_reusable_bind_test | clauses::a_delete_shares_a_handle_across_both_orientations |
| postgres | postgres_reusable_bind_test | clauses::an_insert_select_shares_a_handle_with_its_conflict_assignment |
| postgres | postgres_reusable_bind_test | clauses::an_update_shares_a_handle_between_set_and_where |
| postgres | postgres_reusable_bind_test | clauses::both_union_arms_read_the_same_bound_candidate |
| postgres | postgres_reusable_bind_test | clauses::one_handle_spans_a_projection_two_filters_and_a_grouped_having |
| postgres | postgres_reusable_bind_test | co_change::the_issue_query_binds_16_385_parameters_and_returns_the_right_weight |
| postgres | postgres_reusable_bind_test | co_change::two_handles_over_equal_paths_bind_twice_and_still_agree |
| libsql-only | libsql_reusable_bind_test | clauses::a_cte_body_and_the_outer_where_share_one_placeholder |
| libsql-only | libsql_reusable_bind_test | clauses::a_delete_shares_a_handle_across_both_orientations |
| libsql-only | libsql_reusable_bind_test | clauses::an_update_shares_a_handle_between_set_and_where |
| libsql-only | libsql_reusable_bind_test | clauses::both_union_arms_read_the_same_bound_candidate |
| libsql-only | libsql_reusable_bind_test | clauses::one_handle_spans_a_projection_a_filter_and_an_order_by |
| libsql-only | libsql_reusable_bind_test | co_change::the_issue_query_binds_16_385_parameters_and_returns_the_right_weight |
| libsql-only | libsql_reusable_bind_test | co_change::two_handles_over_equal_paths_bind_twice_and_still_agree |
| rusqlite-only | rusqlite_reusable_bind_test | clauses::a_delete_shares_a_handle_across_both_orientations |
| rusqlite-only | rusqlite_reusable_bind_test | clauses::a_shared_handle_counts_through_the_derived_table_wrap |
| rusqlite-only | rusqlite_reusable_bind_test | clauses::an_insert_select_shares_a_handle_with_its_conflict_assignment |
| rusqlite-only | rusqlite_reusable_bind_test | clauses::an_insert_values_row_shares_a_handle_between_two_columns |
| rusqlite-only | rusqlite_reusable_bind_test | clauses::an_update_shares_a_handle_between_set_and_where |
| rusqlite-only | rusqlite_reusable_bind_test | clauses::negated_shared_predicates_return_the_complementary_rows |
| rusqlite-only | rusqlite_reusable_bind_test | clauses::one_handle_spans_a_projection_and_two_filters |
| rusqlite-only | rusqlite_reusable_bind_test | co_change::a_one_use_handle_binds_exactly_what_the_owned_form_binds |
| rusqlite-only | rusqlite_reusable_bind_test | co_change::the_issue_query_binds_16_385_parameters_and_returns_the_right_weight |
| rusqlite-only | rusqlite_reusable_bind_test | co_change::the_owned_form_exceeds_sqlites_variable_limit_on_the_same_input |
| rusqlite-only | rusqlite_reusable_bind_test | co_change::two_distinct_list_handles_over_the_same_paths_bind_twice |
| rusqlite-only | rusqlite_reusable_bind_test | nesting::a_cte_body_and_the_outer_where_share_one_placeholder |
| rusqlite-only | rusqlite_reusable_bind_test | nesting::a_scalar_subquery_and_the_outer_filter_share_one_placeholder |
| rusqlite-only | rusqlite_reusable_bind_test | nesting::both_union_arms_read_the_same_bound_candidate |
