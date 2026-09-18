# CTEs, set operations, subqueries and table-valued sources

**Feature:** `SelectBuilder` gains `with()` (a `WITH` / `WITH RECURSIVE` prefix built from `Cte`), `union()` / `union_all()`, and — through `TableRef::function` — a table-valued `FROM` / `JOIN` source with bound arguments. `Expr` gains `exists` / `not_exists`, and `Scalar` gains `in_subquery` / `not_in_subquery` / `subquery`, each holding a whole `SelectBuilder` through the object-safe `toolu_orm_core::expr::SelectSource` trait. Parameters number correctly across every statement boundary.
**Drivers:** rusqlite, libsql and Postgres execute the scenarios; `composition_sql_test` and orm-core's `table_function_test` render them per dialect; `facade_only_composition_test` proves the surface composes with `toolu-orm` as a consumer's only dependency.
**Spec:** issue #110.

## What is proven

Seed on every driver:

| Table | Rows |
|---|---|
| `edges(src_kind, src_id, dst_kind, dst_id)` | `a→b`, `b→c`, `c→a`, `c→d` — a three-node **cycle** plus one exit |
| `walk_seeds(batch, kind, id)` | `b1 → a`, `b2 → d` |
| `owners(id)` | `o1`, **NULL** |
| `items(id, owner_id)` | `i1→o1`, `i2→o2` (missing owner), `i3→`**NULL** |
| `code_symbols(id, repo, path)` | `c1/r1/src/a.rs`, `c2/r1/src/b.rs`, `c3/r2/src/a.rs` |
| `code_vec(symbol_id, note)` | `c1/v1`, `c2/v2`, `c3/v3` |

Every claim has a *discriminating* answer: the cycle would loop forever without `UNION`, so the depth limit alone decides the result (`0 → 1` node, `2 → 3`, `3 → 4`); the two NULLs separate `NOT EXISTS` from `NOT IN` twice over; and the two `code_symbols` branches overlap on exactly one id, so `UNION` (3) differs from `UNION ALL` (4) and from either arm (2).

### The recursive graph walk

The issue's `edges_retrieval.rs` statement, built and executed:

```rust
let walk = Cte::new("walk", anchor.union(step))
  .columns(&["kind", "id", "depth"])
  .recursive();
SelectBuilder::from_table(walk.table_ref())
  .column_as(&WALK_ID, "id")
  .column_scalar(Scalar::min(Scalar::col(&WALK_DEPTH)), "depth")
  .group_by(&WALK_ID)
  .order_by(WALK_ID.asc())
  .with(walk)
```

```sql
WITH RECURSIVE "walk"("kind", "id", "depth") AS (
  SELECT "w"."kind" AS "kind", "w"."id" AS "id", CAST(0 AS BIGINT) AS "depth"
    FROM "walk_seeds" AS "w" WHERE "w"."batch" = ?1
  UNION
  SELECT "e"."dst_kind" AS "kind", "e"."dst_id" AS "id", ("s"."depth" + 1) AS "depth"
    FROM "edges" AS "e" INNER JOIN "walk" AS "s"
      ON ("e"."src_kind" = "s"."kind" AND "e"."src_id" = "s"."id")
   WHERE "s"."depth" < ?2)
SELECT "walk"."id" AS "id", MIN("walk"."depth") AS "depth" FROM "walk"
 GROUP BY "walk"."id" ORDER BY "walk"."id" ASC
```

| `max_hops` | Rows | What it proves |
|---|---|---|
| `0` | `a 0` | the seed alone |
| `2` | `a 0`, `b 1`, `c 2` | the walk **returns** — `UNION` collapses `c → a`, so the cycle does not loop |
| `3` | `+ d 3` | the previous result was bounded by the limit, not by the graph |
| `5` | same as `3` | `d` has no outgoing edge, and `a` — reachable again through the cycle — still reports `MIN(depth) = 0` |

`count()` over the walk reports the number of reached nodes (3 and 4), because a CTE builder counts through the derived table with the `WITH` prefix outside it.

The two SQLite suites additionally run the exact production anchor, `FROM json_each(?1) AS "seeds"`, and get the same answers.

### Set operations

| Built from | Result |
|---|---|
| `by_repo("r1").union(by_path("src/a.rs"))` | `c1 c2 c3` — the shared `c1` collapses |
| `…union_all(…)` | `c1 c1 c2 c3` |
| `a.union(b.union(c))` | identical SQL **and** rows to `a.union(b).union(c)` — a nested compound flattens |
| `…order_by(OrderBy::alias_asc("id")).limit(2)` | `c1 c2` — the window bounds the merged set |
| `count()` | `3` for `union`, `4` for `union_all`, `2` for one arm |
| an arm carrying `.order_by(…).limit(…).offset(…)` | byte-identical SQL to the same arm without them |

An arm contributes its select list, source, joins, `WHERE`, `GROUP BY` and `HAVING`. Its own `ORDER BY` / `LIMIT` / `OFFSET` are **not** rendered — SQL has no place for them on an individual arm — and its own CTEs are **hoisted** into the compound's single `WITH` prefix, because dropping them would leave the arm naming a relation nothing declared.

### Subqueries

| Built from | SQL | Rows |
|---|---|---|
| `Expr::not_exists(correlated)` | `WHERE NOT EXISTS (SELECT 1 AS "one" FROM "owners" WHERE "owners"."id" = "items"."owner_id")` | `i2 i3` |
| `Scalar::col(&OWNER).not_in_subquery(all_owner_ids)` | `… NOT IN (SELECT "owners"."id" FROM "owners")` | **`[]`** |
| the same over `WHERE id IS NOT NULL` | — | `i2` — still not `i3` |
| `…in_subquery(…)` | `… IN (SELECT …)` | `i1` |
| `Scalar::subquery(correlated_count)` in a projection | `(SELECT COUNT(*) AS "n" FROM "owners" WHERE …) AS "owner_rows"` | `i1 1`, `i2 0`, `i3 0` |

### Set-based DML

```rust
DeleteBuilder::new("code_vec")
  .filter(Scalar::col(&VEC_SYMBOL_ID).in_subquery(ids_of_repo_path("r1", "src/a.rs")))
```

```sql
DELETE FROM "code_vec" WHERE "code_vec"."symbol_id" IN (
  SELECT "code_symbols"."id" FROM "code_symbols"
   WHERE "code_symbols"."repo" = ?1 AND "code_symbols"."path" = ?2)
```

The parameter vector is exactly `[Text("r1"), Text("src/a.rs")]` — **the id set never crosses into Rust**. The matching row goes, the other repo's rows stay, a subquery matching nothing deletes nothing, and a compound subquery works in the same slot.

### Table-valued sources

`TableRef::function(name, args)` renders `name(?1, …)`: the arguments are bound, the *name* is validated against `[A-Za-z_][A-Za-z0-9_]*` and rejected as `DbCoreError::InvalidTableFunction` otherwise — the same policy `Scalar::func` uses, because a function name is syntax and has no escape.

| Source | Lane | Result |
|---|---|---|
| `json_each(?1)` over `["a","b","c"]` | rusqlite, libsql | three rows; `[]` yields none |
| `json_each(?1)` in a `JOIN` | rusqlite, libsql | its argument binds **before** the `ON` clause |
| `pragma_table_info(?1)` with a bound table name | rusqlite, libsql | the four `edges` column names |
| `regexp_split_to_table($1, $2)` | Postgres | `a b c`; a separator that never occurs yields one row |

### Bind numbering across statement boundaries

One builder carrying a CTE body, a binding projection, a binding `FROM` function, a binding `JOIN … ON`, a `WHERE` subquery, a `UNION` arm and the row window renders as one exact `(sql, params)` pair per dialect, with the indices running `1..=params.len()` — no repeat, no gap:

```sql
WITH "hits" AS (SELECT … WHERE … = ?1) SELECT ?2 AS "tag", … FROM json_each(?3) AS "seeds"
 INNER JOIN "code_symbols" ON (… AND … != ?4)
 WHERE … IN (SELECT … WHERE … = ?5)
 UNION SELECT 'x' AS "tag", … WHERE … = ?6
 ORDER BY "id" ASC LIMIT ?7 OFFSET ?8
```

The rule is one line: **every clause takes its first index from the live `params.len()` of the statement-wide vector and pushes its values as it writes them.** A CTE body and a set-operation arm render into that same vector, so a nested statement continues the count instead of restarting it. `SelectSource::to_select_sql_for(start, dialect)` is the one place that needs an explicit base, and it opens an *offset frame* — pre-fill the vector with the `start - 1` values already emitted, render, split the tail back off — which is the identical computation because the helpers read only `params.len()`.

The derived-table count keeps its #109 behavior of rebuilding from `?1`, because nothing precedes it; the `WITH` prefix is the one thing that renders outside the wrap and therefore takes the low indices.

### Portability notes

- **`json_each` and `pragma_table_info` are SQLite-only *functions*.** The *mechanism* — a bound argument in a `FROM`-clause function call — is portable, and the Postgres suite proves it with `regexp_split_to_table($1, $2)`. `TableRef::function` does not translate names between dialects, exactly as `Scalar::func` does not; naming a function the engine lacks is the caller's error, reported by the engine.
- **`generate_series($1, $2)` is not usable** with untyped parameters: Postgres answers *function generate_series(unknown, unknown) is not unique*. `regexp_split_to_table` infers `{text, text}`.
- **Order a compound by an output name**, `OrderBy::alias_asc("id")`, not by a qualified column. A compound's result has no table qualification: Postgres rejects `ORDER BY "t"."id"` there with *missing FROM-clause entry*, where SQLite tolerates it.
- **`CAST(0 AS BIGINT)`, not a bare `0`, for a recursive depth.** Postgres types a bare literal as `integer`, which makes the whole CTE column `int4` and `MIN(depth)` undecodable into an `i64`. SQLite reads the cast as an ordinary INTEGER.
- **`NOT IN` is not the complement of `IN`** once NULLs are involved, on either engine. `NOT EXISTS` is the NULL-safe form; both behaviors are asserted here so the difference is a pinned fact rather than a surprise.
- **The derived table is always aliased** (`AS "toolu_count"`), because Postgres requires it and SQLite accepts it. A builder using none of these clauses renders byte-identical count and existence SQL.
- **Arms are not type-checked.** Mismatched column counts or types are reported by the engine, not by the builder.
- **`TableRef` no longer derives `Eq`** (it still derives `Debug`, `Clone` and `PartialEq`): a function source holds `Value` arguments, and `Value::Real` holds an `f64`.

### For issue #114

`INSERT … SELECT` needs nothing new: render `INSERT INTO "t" ("a", "b") `, call `SelectSource::to_select_sql_for(params.len() + 1, dialect)` on the source builder, append the returned SQL **unparenthesised**, and extend the parameter vector. The returned statement is already complete and already numbered from the given offset, `WITH` prefix and `UNION` arms included.

## How to run

```sh
cargo nextest run -p toolu-orm-core -E 'binary(table_function_test)'
cargo nextest run -p toolu-orm-query -E 'binary(composition_sql_test)'
cargo nextest run -p toolu-orm-facade-consumer -E 'binary(facade_only_composition_test)'
cargo nextest run -p toolu-orm-query --features rusqlite,sqlite-vec -E 'binary(rusqlite_composition_test)'
cargo nextest run -p toolu-orm-query --features libsql -E 'binary(libsql_composition_test)'
docker compose -f docker-compose.test.yaml up -d --wait
TEST_DB_PORT=5434 cargo nextest run -p toolu-orm-query --features postgres -E 'binary(postgres_composition_test)'
```

## Tests

| Lane | Binary | Test |
|---|---|---|
| default | composition_sql_test | bind_order::every_binding_clause_numbers_in_render_order_on_sqlite |
| default | composition_sql_test | bind_order::the_counted_form_of_the_same_statement_stays_self_consistent |
| default | composition_sql_test | bind_order::the_placeholder_indices_run_from_one_with_no_gap_or_repeat |
| default | composition_sql_test | bind_order::the_same_statement_numbers_with_dollar_placeholders_on_postgres |
| default | composition_sql_test | cte::a_hostile_cte_name_is_quote_doubled_in_the_prefix_and_in_the_from |
| default | composition_sql_test | cte::a_plain_cte_renders_before_select_and_binds_first |
| default | composition_sql_test | cte::an_arms_cte_is_hoisted_into_the_single_with_prefix |
| default | composition_sql_test | cte::an_explicit_column_list_is_quoted_and_parenthesised |
| default | composition_sql_test | cte::several_ctes_render_in_call_order_separated_by_commas |
| default | composition_sql_test | cte::the_outer_statement_numbers_after_every_cte_body |
| default | composition_sql_test | cte::the_recursive_keyword_appears_only_when_a_member_asks_for_it |
| default | composition_sql_test | cte::the_recursive_walk_renders_the_issues_statement |
| default | composition_sql_test | cte::the_same_walk_numbers_with_dollar_placeholders_on_postgres |
| default | composition_sql_test | cte::the_with_prefix_stays_outside_the_count_wrap_and_the_exists |
| default | composition_sql_test | set_ops::a_builder_with_no_arms_renders_the_plain_count_and_exists_unchanged |
| default | composition_sql_test | set_ops::a_compound_count_wraps_the_whole_compound_in_the_derived_table |
| default | composition_sql_test | set_ops::a_compound_exists_wraps_the_compound_not_a_derived_table |
| default | composition_sql_test | set_ops::a_nested_compound_flattens_to_the_same_sql_as_a_chained_one |
| default | composition_sql_test | set_ops::a_union_arm_continues_the_left_arms_bind_numbering |
| default | composition_sql_test | set_ops::an_arms_own_order_by_and_limit_are_not_rendered |
| default | composition_sql_test | set_ops::the_compound_count_is_aliased_on_postgres_too |
| default | composition_sql_test | set_ops::the_same_compound_numbers_with_dollar_placeholders_on_postgres |
| default | composition_sql_test | set_ops::the_tail_renders_after_the_last_arm_and_binds_last |
| default | composition_sql_test | set_ops::union_all_renders_the_all_keyword |
| default | composition_sql_test | subquery::a_compound_subquery_nests_with_all_of_its_own_clauses |
| default | composition_sql_test | subquery::a_correlated_exists_names_the_outer_column_inside_the_subquery |
| default | composition_sql_test | subquery::a_scalar_subquery_projects_a_correlated_count |
| default | composition_sql_test | subquery::a_set_based_delete_binds_only_the_subquerys_own_values |
| default | composition_sql_test | subquery::an_in_subquery_continues_the_outer_bind_numbering |
| default | composition_sql_test | subquery::every_placeholder_index_is_used_exactly_once_in_order |
| default | composition_sql_test | subquery::not_in_renders_the_negated_keyword |
| default | composition_sql_test | subquery::the_positive_form_drops_the_not |
| default | composition_sql_test | subquery::the_same_predicate_numbers_with_dollar_placeholders_on_postgres |
| default | composition_sql_test | table_functions::a_from_function_binds_after_the_select_list |
| default | composition_sql_test | table_functions::a_hostile_function_name_never_reaches_a_statement |
| default | composition_sql_test | table_functions::a_joined_function_binds_before_its_own_on_clause |
| default | composition_sql_test | table_functions::a_pragma_function_source_binds_its_table_name |
| default | composition_sql_test | table_functions::the_count_and_exists_forms_bind_the_source_first |
| default | composition_sql_test | table_functions::the_same_source_numbers_with_dollar_placeholders_on_postgres |
| default | facade_only_composition_test | a_recursive_cte_composes_through_the_facade |
| default | facade_only_composition_test | a_table_valued_source_composes_through_the_facade |
| default | facade_only_composition_test | a_union_with_an_output_ordering_composes_through_the_facade |
| default | facade_only_composition_test | subquery_predicates_compose_through_the_facade |
| default | table_function_test | a_cte_name_is_an_ordinary_relation_source |
| default | table_function_test | a_function_name_that_is_not_a_bare_identifier_is_refused |
| default | table_function_test | a_function_source_numbers_from_the_offset_it_is_given |
| default | table_function_test | a_function_source_renders_its_call_and_binds_its_argument |
| default | table_function_test | a_hostile_alias_is_quote_doubled_rather_than_refused |
| default | table_function_test | a_relation_source_renders_exactly_what_it_always_did |
| default | table_function_test | a_zero_argument_function_renders_empty_parentheses_and_binds_nothing |
| default | table_function_test | an_alias_follows_the_call_and_qualifies_its_columns |
| default | table_function_test | several_arguments_number_consecutively_from_the_offset |
| default | table_function_test | with_alias_agrees_with_the_two_argument_constructor |
| rusqlite-only | composition_sql_test | bind_order::every_binding_clause_numbers_in_render_order_on_sqlite |
| rusqlite-only | composition_sql_test | bind_order::the_counted_form_of_the_same_statement_stays_self_consistent |
| rusqlite-only | composition_sql_test | bind_order::the_placeholder_indices_run_from_one_with_no_gap_or_repeat |
| rusqlite-only | composition_sql_test | bind_order::the_same_statement_numbers_with_dollar_placeholders_on_postgres |
| rusqlite-only | composition_sql_test | cte::a_hostile_cte_name_is_quote_doubled_in_the_prefix_and_in_the_from |
| rusqlite-only | composition_sql_test | cte::a_plain_cte_renders_before_select_and_binds_first |
| rusqlite-only | composition_sql_test | cte::an_arms_cte_is_hoisted_into_the_single_with_prefix |
| rusqlite-only | composition_sql_test | cte::an_explicit_column_list_is_quoted_and_parenthesised |
| rusqlite-only | composition_sql_test | cte::several_ctes_render_in_call_order_separated_by_commas |
| rusqlite-only | composition_sql_test | cte::the_outer_statement_numbers_after_every_cte_body |
| rusqlite-only | composition_sql_test | cte::the_recursive_keyword_appears_only_when_a_member_asks_for_it |
| rusqlite-only | composition_sql_test | cte::the_recursive_walk_renders_the_issues_statement |
| rusqlite-only | composition_sql_test | cte::the_same_walk_numbers_with_dollar_placeholders_on_postgres |
| rusqlite-only | composition_sql_test | cte::the_with_prefix_stays_outside_the_count_wrap_and_the_exists |
| rusqlite-only | composition_sql_test | set_ops::a_builder_with_no_arms_renders_the_plain_count_and_exists_unchanged |
| rusqlite-only | composition_sql_test | set_ops::a_compound_count_wraps_the_whole_compound_in_the_derived_table |
| rusqlite-only | composition_sql_test | set_ops::a_compound_exists_wraps_the_compound_not_a_derived_table |
| rusqlite-only | composition_sql_test | set_ops::a_nested_compound_flattens_to_the_same_sql_as_a_chained_one |
| rusqlite-only | composition_sql_test | set_ops::a_union_arm_continues_the_left_arms_bind_numbering |
| rusqlite-only | composition_sql_test | set_ops::an_arms_own_order_by_and_limit_are_not_rendered |
| rusqlite-only | composition_sql_test | set_ops::the_compound_count_is_aliased_on_postgres_too |
| rusqlite-only | composition_sql_test | set_ops::the_same_compound_numbers_with_dollar_placeholders_on_postgres |
| rusqlite-only | composition_sql_test | set_ops::the_tail_renders_after_the_last_arm_and_binds_last |
| rusqlite-only | composition_sql_test | set_ops::union_all_renders_the_all_keyword |
| rusqlite-only | composition_sql_test | subquery::a_compound_subquery_nests_with_all_of_its_own_clauses |
| rusqlite-only | composition_sql_test | subquery::a_correlated_exists_names_the_outer_column_inside_the_subquery |
| rusqlite-only | composition_sql_test | subquery::a_scalar_subquery_projects_a_correlated_count |
| rusqlite-only | composition_sql_test | subquery::a_set_based_delete_binds_only_the_subquerys_own_values |
| rusqlite-only | composition_sql_test | subquery::an_in_subquery_continues_the_outer_bind_numbering |
| rusqlite-only | composition_sql_test | subquery::every_placeholder_index_is_used_exactly_once_in_order |
| rusqlite-only | composition_sql_test | subquery::not_in_renders_the_negated_keyword |
| rusqlite-only | composition_sql_test | subquery::the_positive_form_drops_the_not |
| rusqlite-only | composition_sql_test | subquery::the_same_predicate_numbers_with_dollar_placeholders_on_postgres |
| rusqlite-only | composition_sql_test | table_functions::a_from_function_binds_after_the_select_list |
| rusqlite-only | composition_sql_test | table_functions::a_hostile_function_name_never_reaches_a_statement |
| rusqlite-only | composition_sql_test | table_functions::a_joined_function_binds_before_its_own_on_clause |
| rusqlite-only | composition_sql_test | table_functions::a_pragma_function_source_binds_its_table_name |
| rusqlite-only | composition_sql_test | table_functions::the_count_and_exists_forms_bind_the_source_first |
| rusqlite-only | composition_sql_test | table_functions::the_same_source_numbers_with_dollar_placeholders_on_postgres |
| rusqlite-only | rusqlite_composition_test | null_sensitive::in_subquery_returns_only_the_matched_key |
| rusqlite-only | rusqlite_composition_test | null_sensitive::not_exists_keeps_the_rows_whose_owner_is_missing_or_null |
| rusqlite-only | rusqlite_composition_test | null_sensitive::not_in_over_a_null_free_set_still_drops_the_row_with_a_null_key |
| rusqlite-only | rusqlite_composition_test | null_sensitive::not_in_over_a_set_containing_null_returns_no_rows_at_all |
| rusqlite-only | rusqlite_composition_test | null_sensitive::the_counted_and_existence_forms_agree_with_the_rows |
| rusqlite-only | rusqlite_composition_test | recursive_walk::a_larger_limit_reaches_the_node_one_hop_further |
| rusqlite-only | rusqlite_composition_test | recursive_walk::a_node_reachable_by_two_paths_reports_its_shallowest_depth |
| rusqlite-only | rusqlite_composition_test | recursive_walk::a_seed_with_no_outgoing_edge_returns_only_itself |
| rusqlite-only | rusqlite_composition_test | recursive_walk::a_zero_limit_returns_only_the_seed |
| rusqlite-only | rusqlite_composition_test | recursive_walk::counting_the_walk_reports_the_number_of_reached_nodes |
| rusqlite-only | rusqlite_composition_test | recursive_walk::the_walk_terminates_on_a_cycle_and_stops_at_the_depth_limit |
| rusqlite-only | rusqlite_composition_test | scalar_subquery::a_scalar_subquery_counts_each_items_owner_rows |
| rusqlite-only | rusqlite_composition_test | scalar_subquery::the_same_projection_survives_a_bounded_first_row_fetch |
| rusqlite-only | rusqlite_composition_test | set_based_dml::a_subquery_matching_nothing_deletes_nothing |
| rusqlite-only | rusqlite_composition_test | set_based_dml::the_delete_removes_only_the_matching_rows |
| rusqlite-only | rusqlite_composition_test | set_based_dml::the_predicate_accepts_a_compound_subquery |
| rusqlite-only | rusqlite_composition_test | set_based_dml::the_statement_binds_only_the_subquerys_own_values |
| rusqlite-only | rusqlite_composition_test | set_ops::a_nested_compound_returns_the_same_rows_as_a_chained_one |
| rusqlite-only | rusqlite_composition_test | set_ops::counting_a_compound_reports_the_deduplicated_size |
| rusqlite-only | rusqlite_composition_test | set_ops::exists_over_a_compound_follows_whether_any_arm_matches |
| rusqlite-only | rusqlite_composition_test | set_ops::the_row_window_applies_to_the_whole_compound |
| rusqlite-only | rusqlite_composition_test | set_ops::union_all_keeps_every_row_of_both_branches |
| rusqlite-only | rusqlite_composition_test | set_ops::union_collapses_the_row_both_branches_return |
| rusqlite-only | rusqlite_composition_test | table_functions::a_function_source_joins_a_real_table |
| rusqlite-only | rusqlite_composition_test | table_functions::a_hostile_alias_is_quoted_not_executed |
| rusqlite-only | rusqlite_composition_test | table_functions::an_empty_json_array_expands_to_no_rows |
| rusqlite-only | rusqlite_composition_test | table_functions::json_each_expands_a_bound_json_array_into_rows |
| rusqlite-only | rusqlite_composition_test | table_functions::pragma_table_info_reads_a_bound_table_name |
| rusqlite-only | rusqlite_composition_test | table_functions::the_production_json_each_seeded_walk_terminates_on_the_cycle |
| libsql-only | composition_sql_test | bind_order::every_binding_clause_numbers_in_render_order_on_sqlite |
| libsql-only | composition_sql_test | bind_order::the_counted_form_of_the_same_statement_stays_self_consistent |
| libsql-only | composition_sql_test | bind_order::the_placeholder_indices_run_from_one_with_no_gap_or_repeat |
| libsql-only | composition_sql_test | bind_order::the_same_statement_numbers_with_dollar_placeholders_on_postgres |
| libsql-only | composition_sql_test | cte::a_hostile_cte_name_is_quote_doubled_in_the_prefix_and_in_the_from |
| libsql-only | composition_sql_test | cte::a_plain_cte_renders_before_select_and_binds_first |
| libsql-only | composition_sql_test | cte::an_arms_cte_is_hoisted_into_the_single_with_prefix |
| libsql-only | composition_sql_test | cte::an_explicit_column_list_is_quoted_and_parenthesised |
| libsql-only | composition_sql_test | cte::several_ctes_render_in_call_order_separated_by_commas |
| libsql-only | composition_sql_test | cte::the_outer_statement_numbers_after_every_cte_body |
| libsql-only | composition_sql_test | cte::the_recursive_keyword_appears_only_when_a_member_asks_for_it |
| libsql-only | composition_sql_test | cte::the_recursive_walk_renders_the_issues_statement |
| libsql-only | composition_sql_test | cte::the_same_walk_numbers_with_dollar_placeholders_on_postgres |
| libsql-only | composition_sql_test | cte::the_with_prefix_stays_outside_the_count_wrap_and_the_exists |
| libsql-only | composition_sql_test | set_ops::a_builder_with_no_arms_renders_the_plain_count_and_exists_unchanged |
| libsql-only | composition_sql_test | set_ops::a_compound_count_wraps_the_whole_compound_in_the_derived_table |
| libsql-only | composition_sql_test | set_ops::a_compound_exists_wraps_the_compound_not_a_derived_table |
| libsql-only | composition_sql_test | set_ops::a_nested_compound_flattens_to_the_same_sql_as_a_chained_one |
| libsql-only | composition_sql_test | set_ops::a_union_arm_continues_the_left_arms_bind_numbering |
| libsql-only | composition_sql_test | set_ops::an_arms_own_order_by_and_limit_are_not_rendered |
| libsql-only | composition_sql_test | set_ops::the_compound_count_is_aliased_on_postgres_too |
| libsql-only | composition_sql_test | set_ops::the_same_compound_numbers_with_dollar_placeholders_on_postgres |
| libsql-only | composition_sql_test | set_ops::the_tail_renders_after_the_last_arm_and_binds_last |
| libsql-only | composition_sql_test | set_ops::union_all_renders_the_all_keyword |
| libsql-only | composition_sql_test | subquery::a_compound_subquery_nests_with_all_of_its_own_clauses |
| libsql-only | composition_sql_test | subquery::a_correlated_exists_names_the_outer_column_inside_the_subquery |
| libsql-only | composition_sql_test | subquery::a_scalar_subquery_projects_a_correlated_count |
| libsql-only | composition_sql_test | subquery::a_set_based_delete_binds_only_the_subquerys_own_values |
| libsql-only | composition_sql_test | subquery::an_in_subquery_continues_the_outer_bind_numbering |
| libsql-only | composition_sql_test | subquery::every_placeholder_index_is_used_exactly_once_in_order |
| libsql-only | composition_sql_test | subquery::not_in_renders_the_negated_keyword |
| libsql-only | composition_sql_test | subquery::the_positive_form_drops_the_not |
| libsql-only | composition_sql_test | subquery::the_same_predicate_numbers_with_dollar_placeholders_on_postgres |
| libsql-only | composition_sql_test | table_functions::a_from_function_binds_after_the_select_list |
| libsql-only | composition_sql_test | table_functions::a_hostile_function_name_never_reaches_a_statement |
| libsql-only | composition_sql_test | table_functions::a_joined_function_binds_before_its_own_on_clause |
| libsql-only | composition_sql_test | table_functions::a_pragma_function_source_binds_its_table_name |
| libsql-only | composition_sql_test | table_functions::the_count_and_exists_forms_bind_the_source_first |
| libsql-only | composition_sql_test | table_functions::the_same_source_numbers_with_dollar_placeholders_on_postgres |
| libsql-only | libsql_composition_test | null_sensitive::in_subquery_returns_only_the_matched_key |
| libsql-only | libsql_composition_test | null_sensitive::not_exists_keeps_the_rows_whose_owner_is_missing_or_null |
| libsql-only | libsql_composition_test | null_sensitive::not_in_over_a_null_free_set_still_drops_the_row_with_a_null_key |
| libsql-only | libsql_composition_test | null_sensitive::not_in_over_a_set_containing_null_returns_no_rows_at_all |
| libsql-only | libsql_composition_test | null_sensitive::the_counted_and_existence_forms_agree_with_the_rows |
| libsql-only | libsql_composition_test | recursive_walk::a_larger_limit_reaches_the_node_one_hop_further |
| libsql-only | libsql_composition_test | recursive_walk::a_node_reachable_by_two_paths_reports_its_shallowest_depth |
| libsql-only | libsql_composition_test | recursive_walk::a_seed_with_no_outgoing_edge_returns_only_itself |
| libsql-only | libsql_composition_test | recursive_walk::a_zero_limit_returns_only_the_seed |
| libsql-only | libsql_composition_test | recursive_walk::counting_the_walk_reports_the_number_of_reached_nodes |
| libsql-only | libsql_composition_test | recursive_walk::the_walk_terminates_on_a_cycle_and_stops_at_the_depth_limit |
| libsql-only | libsql_composition_test | scalar_subquery::a_scalar_subquery_counts_each_items_owner_rows |
| libsql-only | libsql_composition_test | scalar_subquery::the_same_projection_survives_a_bounded_first_row_fetch |
| libsql-only | libsql_composition_test | set_based_dml::a_subquery_matching_nothing_deletes_nothing |
| libsql-only | libsql_composition_test | set_based_dml::the_delete_removes_only_the_matching_rows |
| libsql-only | libsql_composition_test | set_based_dml::the_predicate_accepts_a_compound_subquery |
| libsql-only | libsql_composition_test | set_based_dml::the_statement_binds_only_the_subquerys_own_values |
| libsql-only | libsql_composition_test | set_ops::a_nested_compound_returns_the_same_rows_as_a_chained_one |
| libsql-only | libsql_composition_test | set_ops::counting_a_compound_reports_the_deduplicated_size |
| libsql-only | libsql_composition_test | set_ops::exists_over_a_compound_follows_whether_any_arm_matches |
| libsql-only | libsql_composition_test | set_ops::the_row_window_applies_to_the_whole_compound |
| libsql-only | libsql_composition_test | set_ops::union_all_keeps_every_row_of_both_branches |
| libsql-only | libsql_composition_test | set_ops::union_collapses_the_row_both_branches_return |
| libsql-only | libsql_composition_test | table_functions::a_function_source_joins_a_real_table |
| libsql-only | libsql_composition_test | table_functions::a_hostile_alias_is_quoted_not_executed |
| libsql-only | libsql_composition_test | table_functions::an_empty_json_array_expands_to_no_rows |
| libsql-only | libsql_composition_test | table_functions::json_each_expands_a_bound_json_array_into_rows |
| libsql-only | libsql_composition_test | table_functions::pragma_table_info_reads_a_bound_table_name |
| libsql-only | libsql_composition_test | table_functions::the_production_json_each_seeded_walk_terminates_on_the_cycle |
| postgres | composition_sql_test | bind_order::every_binding_clause_numbers_in_render_order_on_sqlite |
| postgres | composition_sql_test | bind_order::the_counted_form_of_the_same_statement_stays_self_consistent |
| postgres | composition_sql_test | bind_order::the_placeholder_indices_run_from_one_with_no_gap_or_repeat |
| postgres | composition_sql_test | bind_order::the_same_statement_numbers_with_dollar_placeholders_on_postgres |
| postgres | composition_sql_test | cte::a_hostile_cte_name_is_quote_doubled_in_the_prefix_and_in_the_from |
| postgres | composition_sql_test | cte::a_plain_cte_renders_before_select_and_binds_first |
| postgres | composition_sql_test | cte::an_arms_cte_is_hoisted_into_the_single_with_prefix |
| postgres | composition_sql_test | cte::an_explicit_column_list_is_quoted_and_parenthesised |
| postgres | composition_sql_test | cte::several_ctes_render_in_call_order_separated_by_commas |
| postgres | composition_sql_test | cte::the_outer_statement_numbers_after_every_cte_body |
| postgres | composition_sql_test | cte::the_recursive_keyword_appears_only_when_a_member_asks_for_it |
| postgres | composition_sql_test | cte::the_recursive_walk_renders_the_issues_statement |
| postgres | composition_sql_test | cte::the_same_walk_numbers_with_dollar_placeholders_on_postgres |
| postgres | composition_sql_test | cte::the_with_prefix_stays_outside_the_count_wrap_and_the_exists |
| postgres | composition_sql_test | set_ops::a_builder_with_no_arms_renders_the_plain_count_and_exists_unchanged |
| postgres | composition_sql_test | set_ops::a_compound_count_wraps_the_whole_compound_in_the_derived_table |
| postgres | composition_sql_test | set_ops::a_compound_exists_wraps_the_compound_not_a_derived_table |
| postgres | composition_sql_test | set_ops::a_nested_compound_flattens_to_the_same_sql_as_a_chained_one |
| postgres | composition_sql_test | set_ops::a_union_arm_continues_the_left_arms_bind_numbering |
| postgres | composition_sql_test | set_ops::an_arms_own_order_by_and_limit_are_not_rendered |
| postgres | composition_sql_test | set_ops::the_compound_count_is_aliased_on_postgres_too |
| postgres | composition_sql_test | set_ops::the_same_compound_numbers_with_dollar_placeholders_on_postgres |
| postgres | composition_sql_test | set_ops::the_tail_renders_after_the_last_arm_and_binds_last |
| postgres | composition_sql_test | set_ops::union_all_renders_the_all_keyword |
| postgres | composition_sql_test | subquery::a_compound_subquery_nests_with_all_of_its_own_clauses |
| postgres | composition_sql_test | subquery::a_correlated_exists_names_the_outer_column_inside_the_subquery |
| postgres | composition_sql_test | subquery::a_scalar_subquery_projects_a_correlated_count |
| postgres | composition_sql_test | subquery::a_set_based_delete_binds_only_the_subquerys_own_values |
| postgres | composition_sql_test | subquery::an_in_subquery_continues_the_outer_bind_numbering |
| postgres | composition_sql_test | subquery::every_placeholder_index_is_used_exactly_once_in_order |
| postgres | composition_sql_test | subquery::not_in_renders_the_negated_keyword |
| postgres | composition_sql_test | subquery::the_positive_form_drops_the_not |
| postgres | composition_sql_test | subquery::the_same_predicate_numbers_with_dollar_placeholders_on_postgres |
| postgres | composition_sql_test | table_functions::a_from_function_binds_after_the_select_list |
| postgres | composition_sql_test | table_functions::a_hostile_function_name_never_reaches_a_statement |
| postgres | composition_sql_test | table_functions::a_joined_function_binds_before_its_own_on_clause |
| postgres | composition_sql_test | table_functions::a_pragma_function_source_binds_its_table_name |
| postgres | composition_sql_test | table_functions::the_count_and_exists_forms_bind_the_source_first |
| postgres | composition_sql_test | table_functions::the_same_source_numbers_with_dollar_placeholders_on_postgres |
| postgres | postgres_composition_test | null_sensitive::in_subquery_returns_only_the_matched_key |
| postgres | postgres_composition_test | null_sensitive::not_exists_keeps_the_rows_whose_owner_is_missing_or_null |
| postgres | postgres_composition_test | null_sensitive::not_in_over_a_null_free_set_still_drops_the_row_with_a_null_key |
| postgres | postgres_composition_test | null_sensitive::not_in_over_a_set_containing_null_returns_no_rows_at_all |
| postgres | postgres_composition_test | null_sensitive::the_counted_and_existence_forms_agree_with_the_rows |
| postgres | postgres_composition_test | recursive_walk::a_larger_limit_reaches_the_node_one_hop_further |
| postgres | postgres_composition_test | recursive_walk::a_node_reachable_by_two_paths_reports_its_shallowest_depth |
| postgres | postgres_composition_test | recursive_walk::a_seed_with_no_outgoing_edge_returns_only_itself |
| postgres | postgres_composition_test | recursive_walk::a_zero_limit_returns_only_the_seed |
| postgres | postgres_composition_test | recursive_walk::counting_the_walk_reports_the_number_of_reached_nodes |
| postgres | postgres_composition_test | recursive_walk::the_walk_terminates_on_a_cycle_and_stops_at_the_depth_limit |
| postgres | postgres_composition_test | scalar_subquery::a_scalar_subquery_counts_each_items_owner_rows |
| postgres | postgres_composition_test | scalar_subquery::the_same_projection_survives_a_bounded_first_row_fetch |
| postgres | postgres_composition_test | set_based_dml::a_subquery_matching_nothing_deletes_nothing |
| postgres | postgres_composition_test | set_based_dml::the_delete_removes_only_the_matching_rows |
| postgres | postgres_composition_test | set_based_dml::the_predicate_accepts_a_compound_subquery |
| postgres | postgres_composition_test | set_based_dml::the_statement_binds_only_the_subquerys_own_values |
| postgres | postgres_composition_test | set_ops::a_nested_compound_returns_the_same_rows_as_a_chained_one |
| postgres | postgres_composition_test | set_ops::counting_a_compound_reports_the_deduplicated_size |
| postgres | postgres_composition_test | set_ops::exists_over_a_compound_follows_whether_any_arm_matches |
| postgres | postgres_composition_test | set_ops::the_row_window_applies_to_the_whole_compound |
| postgres | postgres_composition_test | set_ops::union_all_keeps_every_row_of_both_branches |
| postgres | postgres_composition_test | set_ops::union_collapses_the_row_both_branches_return |
| postgres | postgres_composition_test | table_functions::a_separator_that_does_not_occur_yields_one_row |
| postgres | postgres_composition_test | table_functions::a_table_valued_source_expands_its_bound_arguments_into_rows |
| postgres | postgres_composition_test | table_functions::the_source_renders_both_arguments_as_bound_parameters |
