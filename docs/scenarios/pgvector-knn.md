# pgvector KNN (`<->` / `<=>` / `<#>`)

**Feature:** `PgVectorOps` on `Column<Vector>` builds `"col" <-> '[…]'::vector`
(and `<=>` / `<#>`). `PgVectorDistance` is selectable via `column_expr` and
orderable via `.asc()` / `Into<OrderBy>`. Pair with `limit(k)` for the
`ORDER BY … LIMIT k` shape. [vec0 KNN](vec0-knn.md) is the SQLite sqlite-vec
surface — these do not share APIs. [Postgres FTS](postgres-fts-queries.md) is
unrelated.
**Drivers:** Postgres only. On SQLite every constructor refuses.
**Spec:** [pgvector query surface](../toolu/specs/2026-09-09-pgvector-query-surface-design.md), AC-1 … AC-7.

## What is proven

### The shape that motivated this

```rust
let dist = items::embedding.l2_distance_for(Dialect::Postgres, &[0.1, 0.2, 0.3])?;

let (sql, params) = SelectBuilder::new("items")
  .columns_raw(&["id"])
  .column_expr(dist.sql(), "distance")
  .order_by(OrderBy::alias_asc("distance"))
  .limit(5)
  .to_sql_for(Dialect::Postgres);
```

```sql
SELECT "id", "items"."embedding" <-> '[0.1,0.2,0.3]'::vector AS "distance"
  FROM "items"
 ORDER BY "distance" ASC
 LIMIT $1
```

The query vector is embedded as a SQL literal (finite `&[f32]` only) because
select-list / `ORDER BY` carry no bind params today — same constraint as
`pg_fts::ts_rank`. `LIMIT` remains bound. No `MATCH` / `"k"` appears.

### Operators

| Method | SQL op | ASC means |
|---|---|---|
| `l2_distance` | `<->` | nearest (Euclidean) |
| `cosine_distance` | `<=>` | nearest (cosine distance) |
| `neg_inner_product` | `<#>` | nearest (neg IP / highest similarity) |

### Extension and compose image

Live tests need the **pgvector** extension. `docker-compose.test.yaml` uses
`pgvector/pgvector:pg16` (not stock `postgres:16-alpine`). Suites run
`CREATE EXTENSION IF NOT EXISTS vector` and **fail hard** if the extension is
unavailable — they never skip.

### SQLite

Every constructor refuses at construction:

```
l2_distance is a Postgres pgvector feature with no sqlite equivalent; build this
query for Postgres, or use the sqlite-vec KNN surface (MATCH / k / distance) instead
```

sqlite-vec `.knn` still refuses Postgres (`Vec0UnsupportedDialect`).

## Tests

| Lane | Binary | Test |
|---|---|---|
| default / postgres | `pgvector_query_test` | `l2_embeds_the_query_vector_as_a_literal` |
| default / postgres | `pgvector_query_test` | `cosine_and_neg_inner_product_emit_their_ops` |
| default / postgres | `pgvector_query_test` | `free_functions_agree_with_the_ops_trait` |
| default / postgres | `pgvector_query_test` | `the_short_forms_agree_with_the_current_dialect` |
| default / postgres | `pgvector_query_test` | `extreme_finite_floats_never_use_scientific_notation` |
| default / postgres | `pgvector_query_test` | `sqlite_is_refused_by_every_constructor` |
| default / postgres | `pgvector_query_test` | `a_non_finite_embedding_element_is_refused` |
| default / postgres | `pgvector_query_test` | `vec0_still_refuses_postgres_and_pgvector_refuses_sqlite` |
| default / postgres | `pgvector_select_sql_test` | `the_knn_shape_from_the_issue_is_expressible` |
| default / postgres | `pgvector_select_sql_test` | `a_query_can_order_by_distance_without_selecting_it` |
| default / postgres | `pgvector_select_sql_test` | `cosine_and_neg_inner_product_ops_reach_the_builder` |
| default / postgres | `pgvector_select_sql_test` | `sqlite_distance_is_refused_before_any_sql` |
| postgres | `pgvector_postgres_test` | `l2_top_k_returns_nearest_neighbour_first` |
| postgres | `pgvector_postgres_test` | `order_by_distance_without_projection_still_ranks` |
| postgres | `pgvector_postgres_test` | `cosine_and_neg_inner_product_run_on_live_server` |
