# Table aliases and JOIN predicates

**Feature:** `TableRef` puts a table in a `FROM` / `JOIN` slot under an optional alias, `TableRef::column` turns a typed `Column<T>` into an `AliasedColumn<T>` addressed through that alias, `columns_qualified` / `column_as` project qualified, and `JoinCondition` is an expression tree — column-to-column comparisons and bound-value predicates combined with `and` / `or`.
**Drivers:** libsql, rusqlite, Postgres. The rendered SQL is pinned per dialect without a database; the four behaviours are executed against real rows on all three.
**Spec:** issue [#111](https://github.com/Falconiere/toolu-orm/issues/111).

## What is proven

```rust
let c = TableRef::aliased("code_symbols", "c");
let f = TableRef::aliased("code_feedback", "f");

SelectBuilder::from_table(&c)
  .column_as(&c.column(&SYMBOL_ID), "c_id")
  .column_as(&f.column(&FEEDBACK_ID), "f_id")
  .left_join(
    &f,
    f.column(&REPO).equals(&c.column(&REPO))
      .and(f.column(&PATH).equals(&c.column(&PATH)))
      .and(f.column(&LIVE).eq(1)),
  )
  .filter(c.column(&KIND).eq("note"))
```

renders, on SQLite (Postgres is identical with `$N`):

```sql
SELECT "c"."id" AS "c_id", "f"."id" AS "f_id"
  FROM "code_symbols" AS "c"
  LEFT JOIN "code_feedback" AS "f"
    ON (("f"."repo" = "c"."repo" AND "f"."path" = "c"."path") AND "f"."live" = ?1)
 WHERE "c"."kind" = ?2
```

Seed shared by all three drivers — `code_symbols(id, owner, kind, created_at)` and
`code_feedback(id, symbol_id, live, label)`, both carrying an `id`:

| code_symbols | owner | kind | created_at | | code_feedback | symbol_id | live | label |
|---|---|---|---|---|---|---|---|---|
| s1 | o1 | note | 10 | | f1 | s1 | 1 | up |
| s2 | o1 | note | 20 | | f2 | s2 | 1 | down |
| s3 | o2 | task | 30 | | f3 | s3 | 1 | up |
| s4 | o1 | note | 40 | | f4 | s4 | **0** | stale |

| Scenario | Result |
|---|---|
| Self-join: `code_symbols AS old` ⋈ `code_symbols AS newer` on `old.owner = newer.owner AND old.created_at < newer.created_at` | `(s1,s2) (s1,s4) (s2,s4)`; `o2` holds one row, so it pairs with nothing |
| Same table joined twice: `code_feedback AS up` (`label = ?1`) and `code_feedback AS down` (`label = ?2`) | one row per symbol, `s1 up/-`, `s2 -/down`, `s3 up/-`, `s4 -/-`; the second `ON` numbers its placeholder after the first |
| Duplicate column names: `columns_typed(&[&S_ID, &F_LABEL])` after the join | rejected by the driver, "ambiguous column name" / "column reference is ambiguous" |
| The same query with `column_as(&c.column(&S_ID), "s_id")` / `column_as(&f.column(&F_ID), "f_id")` | 4 rows decoded, `s_id != f_id` on every row |
| `LEFT JOIN … ON f.symbol_id = c.id AND f.live = ?1` | **4 rows**: `s4` is kept with a `NULL` label, because its only feedback row is `live = 0` |
| the same predicate moved to `.filter(...)` (`WHERE`) | **3 rows**: `s4` is dropped — which is why an `ON` predicate is not a `WHERE` predicate |
| `ON f.live = ?1` + `WHERE c.kind = ?2`, as `(1, "note")` / `(0, "note")` / `(1, "task")` | `s1 s2 s4` with `up/down/NULL`; `s1 s2 s4` with `NULL/NULL/stale`; `s3` — changing either parameter alone changes only its own clause |
| `count` / `exists` on the builder that binds in both clauses | agree with `fetch_all().len()`; join parameters reach `to_count_sql_for` and `to_exists_sql_for`, which never received any before |
| identifier escaping | `TableRef::aliased("me\"m", "o\"ld")` renders `"me""m" AS "o""ld"` and its columns as `"o""ld"."id"`; an alias of `x"; DROP TABLE code_symbols; --` is *executed* on rusqlite and returns all four rows with the table intact |
| `columns_typed`, and `join("pipelines", a.equals(&b))` with a `&str` table | unchanged, byte for byte |

`JoinCondition` no longer has a parameterless `to_sql()`: an `ON` clause may
bind values, so it renders through `to_sql_fragment_for(start, dialect)` like
every other expression, and `Expr` converts into it and back.

## How to run

```sh
cargo nextest run -p toolu-orm-core -E 'binary(table_alias_test)'
cargo nextest run -p toolu-orm-query -E 'binary(join_alias_sql_test)'
cargo nextest run -p toolu-orm-query --features libsql -E 'binary(libsql_joins_test)'
cargo nextest run -p toolu-orm-query --features rusqlite -E 'binary(rusqlite_joins_test)'
docker compose -f docker-compose.test.yaml up -d --wait
TEST_DB_PORT=5434 cargo nextest run -p toolu-orm -p toolu-orm-core -p toolu-orm-macros -p toolu-orm-query -p toolu-orm-connection -p toolu-orm-cli -p toolu-orm-facade-consumer --features postgres -E 'binary(postgres_joins_test)'
```

## Tests

| Lane | Binary | Test |
|---|---|---|
| default | table_alias_test | table_ref_renders_alias_and_doubles_embedded_quotes |
| default | table_alias_test | aliased_column_qualifies_with_the_alias_and_falls_back_to_the_table |
| default | table_alias_test | aliased_column_supports_the_same_predicate_and_ordering_ops |
| default | table_alias_test | column_to_column_comparisons_render_every_operator_without_params |
| default | table_alias_test | join_condition_and_or_number_params_from_the_given_start |
| default | join_alias_sql_test | alias_tests::aliased_from_and_join_render_table_as_alias |
| default | join_alias_sql_test | alias_tests::str_table_join_renders_exactly_what_it_rendered_before |
| default | join_alias_sql_test | alias_tests::compound_on_clause_binds_its_value_and_parenthesizes |
| default | join_alias_sql_test | alias_tests::on_or_clause_renders_or_with_parentheses |
| default | join_alias_sql_test | param_order_tests::on_params_precede_where_params_sqlite |
| default | join_alias_sql_test | param_order_tests::on_params_precede_where_params_postgres |
| default | join_alias_sql_test | param_order_tests::count_exists_and_first_row_thread_join_params |
| default | join_alias_sql_test | alias_tests::qualified_projection_and_output_alias_render_qualified |
| rusqlite-only | rusqlite_joins_test | self_join_under_two_aliases_returns_expected_pairs |
| rusqlite-only | rusqlite_joins_test | same_table_joined_twice_projects_both_labels |
| rusqlite-only | rusqlite_joins_test | unqualified_projection_is_ambiguous_but_qualified_one_decodes |
| rusqlite-only | rusqlite_joins_test | left_join_on_predicate_keeps_unmatched_rows_where_drops_them |
| rusqlite-only | rusqlite_joins_test | on_and_where_params_select_the_expected_rows |
| rusqlite-only | rusqlite_joins_test | an_injection_shaped_alias_is_quoted_not_executed |
| libsql-only | libsql_joins_test | self_join_under_two_aliases_returns_expected_pairs |
| libsql-only | libsql_joins_test | same_table_joined_twice_projects_both_labels |
| libsql-only | libsql_joins_test | unqualified_projection_is_ambiguous_but_qualified_one_decodes |
| libsql-only | libsql_joins_test | left_join_on_predicate_keeps_unmatched_rows_where_drops_them |
| libsql-only | libsql_joins_test | on_and_where_params_select_the_expected_rows |
| postgres | postgres_joins_test | self_join_under_two_aliases_returns_expected_pairs |
| postgres | postgres_joins_test | same_table_joined_twice_projects_both_labels |
| postgres | postgres_joins_test | unqualified_projection_is_ambiguous_but_qualified_one_decodes |
| postgres | postgres_joins_test | left_join_on_predicate_keeps_unmatched_rows_where_drops_them |
| postgres | postgres_joins_test | on_and_where_params_select_the_expected_rows |
