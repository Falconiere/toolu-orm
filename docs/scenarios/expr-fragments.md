# Expression fragments

**Feature:** `Expr::to_sql_fragment_for(start, dialect)` renders a WHERE fragment whose placeholders continue from `start`, so builders can chain filters, joins, and raw SQL without renumbering. SQLite gets `?N`, Postgres gets `$N`.
**Drivers:** both dialects, pure SQL generation (no database). The same operators are executed against real rows in [Filters](filters.md).
**Spec:** AC-11.

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

The empty-list rows pin a fix made by this program: the renderer used to emit `IN ()`, which Postgres rejects with SQLSTATE 42601 (SQLite silently treated it as false). Drizzle renders the same constants.

The `Expr::raw` row pins the fix for issue #113: a raw fragment nested inside `and`/`or` used to number its bare `?` placeholders from the caller's `start` alone, colliding with placeholders already emitted by earlier siblings in the same tree, instead of continuing from `start + <params already emitted>`.

## How to run

```sh
cargo nextest run -p toolu-orm-core -E 'binary(expr_offset_and_nesting_test) | binary(expr_test) | binary(expr_raw_bind_index_test)'
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
