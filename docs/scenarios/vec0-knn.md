# vec0 KNN queries

**Feature:** `SelectBuilder::knn` emits `embedding MATCH ? AND k = ?` as
top-level WHERE conjuncts; `Vec0Ops` on `Column<Vector>` builds the MATCH
half; `vec0::k_eq` builds the hidden scan-size parameter; `vec0::distance`
is the synthesised output column, selectable and orderable. Every entry
point refuses `Dialect::Postgres`.
**Drivers:** pure-SQL generation on the default and postgres compile
lanes. Live execution is the rusqlite lane with the optional `sqlite-vec`
feature on `toolu-orm-query`: CI statically links the official `sqlite-vec`
crate, registers it, adopts the connection via `from_connection`, and runs
ORM `vec0` DDL plus `SelectBuilder::knn` end-to-end (see
[vec0 virtual tables](vec0-virtual-tables.md)).
**Spec:** [vec0 KNN](../toolu/specs/2026-09-08-vec0-knn-design.md), AC-1 … AC-8;
live lane [2026-09-09](../toolu/specs/2026-09-09-live-sqlite-vec-tests-design.md).

**Reading one:** [vec0 virtual tables](vec0-virtual-tables.md) declares the
index; this page searches it.

## What is proven

### The shape of a KNN query

```rust
let q = Value::vector_with_dim(&query_vec, 1024)?;
let (sql, params) = SelectBuilder::new(memory_vec::TABLE)
    .columns_raw(&["memory_id", "distance"])
    .knn(&memory_vec::embedding, q, k)?
    .join(memories::TABLE, memories::id.equals(&memory_vec::memory_id))
    .filter(memories::deleted_at.is_null())
    .order_by(vec0::distance()?)
    .limit(limit)
    .to_sql();
```

renders (parameter indices illustrative):

```sql
SELECT "memory_id", "distance"
  FROM "memory_vec"
  INNER JOIN "memories" ON "memories"."id" = "memory_vec"."memory_id"
 WHERE "memory_vec"."embedding" MATCH ?1
   AND "k" = ?2
   AND "memories"."deleted_at" IS NULL
 ORDER BY "distance" ASC
 LIMIT ?3
```

`k` is a hidden column of the `vec0` module — a parameter of the index
scan, not a filter. `.knn` pushes it as a top-level `AND` sibling of the
`MATCH` so it cannot be nested under an `OR`. A second `.knn` on the same
builder is refused.

### Filtered KNN needs oversample

When other predicates narrow the result, `k` neighbours are fetched
**then** filtered. A filtered search with `k == limit` quietly returns
fewer rows than asked for. Pass a `k` larger than `LIMIT` and re-trim —
the builder does not invent the larger `k` for you.

### Postgres is refused, not translated

pgvector’s `<->` / `<=>` / `<#>` and "ORDER BY distance LIMIT k" are a
different model. Every vec0 constructor (`matches_for`, `k_eq_for`,
`distance_for`, `knn_for`) returns
`DbCoreError::Vec0UnsupportedDialect` for `Dialect::Postgres` and
produces no SQL. Short forms follow `Dialect::CURRENT`.

The Postgres surface lives in [pgvector KNN](pgvector-knn.md) — a separate
module (`toolu_orm_core::pgvector`), not a translation of this one.

### Extension required to execute

Generating the SQL does not need `sqlite-vec`. Running it does: register
the extension on the connection before preparing the statement (and before
`run_migrate` for the DDL that creates the table). Without it the driver
fails with a missing-module error.

The rusqlite CI lane enables `--features rusqlite,sqlite-vec` on
`toolu-orm-query` (and the register feature on connection), which statically
links the extension and runs `vec0_sqlite_vec_live_test` through
`RusqliteConnection::from_connection` plus `SelectBuilder::knn`.

## Tests

| Lane | Binary | Test |
|---|---|---|
| default | vec0_knn_test | rendering::vector_match_qualifies_the_column_and_binds_the_blob |
| default | vec0_knn_test | rendering::vector_match_continues_the_callers_parameter_numbering |
| default | vec0_knn_test | rendering::k_eq_renders_the_quoted_hidden_column |
| default | vec0_knn_test | rendering::distance_renders_as_a_quoted_identifier |
| default | vec0_knn_test | rendering::distance_converts_to_ascending_order_by_default |
| default | vec0_knn_test | rendering::the_short_forms_agree_with_the_current_dialect |
| default | vec0_knn_test | rejections::postgres_is_refused_by_match_k_and_distance |
| default | vec0_knn_test | rejections::non_positive_k_is_refused |
| default | vec0_knn_sql_test | knn_emits_match_and_k_as_top_level_conjuncts |
| default | vec0_knn_sql_test | an_extra_filter_stays_a_sibling_of_match_and_k |
| default | vec0_knn_sql_test | the_knn_query_from_the_issue_is_expressible |
| default | vec0_knn_sql_test | columns_raw_can_select_distance_without_an_alias |
| default | vec0_knn_sql_test | postgres_knn_is_refused |
| default | vec0_knn_sql_test | non_positive_k_is_refused_by_knn |
| default | vec0_knn_sql_test | a_second_knn_on_the_same_builder_is_refused |
| default | vec0_knn_sql_test | the_short_knn_agrees_with_the_current_dialect |
| rusqlite-only | vec0_sqlite_vec_live_test | from_connection_keeps_sqlite_vec_so_vec0_ddl_applies |
| rusqlite-only | vec0_sqlite_vec_live_test | knn_returns_the_nearest_seeded_row_first |
| rusqlite-only | vec0_sqlite_vec_live_test | knn_on_an_empty_vec0_table_returns_no_rows |
