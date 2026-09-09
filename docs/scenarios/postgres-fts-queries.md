# Postgres FTS queries (`@@`, `to_tsquery`, `ts_rank`)

**Feature:** `pg_fts::column` / `to_tsvector` build a `PgTsDocument`; `matches_tsquery` / `matches_plainto_tsquery` / `matches_websearch_to_tsquery` put `@@` in `filter(...)`; `pg_fts::ts_rank_*` renders `ts_rank` as `PgFtsFn` for `column_expr` and `order_by`. [FTS5 queries](fts5-queries.md) is the SQLite surface — these do not share APIs.
**Drivers:** Postgres only. On SQLite every constructor refuses.
**Spec:** [Postgres FTS query surface](../toolu/specs/2026-09-09-postgres-fts-query-surface-design.md), AC-1 … AC-9.

## What is proven

### The query that motivated this

```rust
let doc = pg_fts::column_for(Dialect::Postgres, &SEARCH_VECTOR)?;
let score = pg_fts::ts_rank_tsquery_for(
  Dialect::Postgres, &doc, "english", "runner", None,
)?;

let hits: Vec<ScoredHit> = SelectBuilder::new("docs")
  .columns_raw(&["id"])
  .column_expr(score.sql(), "score")
  .filter(doc.matches_tsquery_for(Dialect::Postgres, "english", "runner")?)
  .filter(DELETED_AT.is_null())
  .order_by(OrderBy::alias_desc("score"))
  .limit(10)
  .fetch_all(&conn)
  .await?;
```

```sql
SELECT "id", ts_rank("docs"."search_vector", to_tsquery('english', 'runner')) AS "score"
  FROM "docs"
 WHERE "docs"."search_vector" @@ to_tsquery('english', $1)
   AND "docs"."deleted_at" IS NULL
 ORDER BY "score" DESC LIMIT $2
```

`@@` binds the pattern; `ts_rank` embeds it as an escaped literal (select-list / `ORDER BY` carry no params today). No `MATCH` / `bm25` appears.

### `ts_rank` is positive, so `DESC` is best first

Postgres relevance scores are **positive**, and a better match is **higher**. `ORDER BY score DESC` returns the best match first — the inverse of FTS5 `bm25` (negative / `ASC`).

### Weights are first, and they are validated

`ts_rank(real[], tsvector, tsquery)` takes D/C/B/A weights as the **first** argument. Optional `&[f32; 4]` renders `'{0.0,0.0,0.0,1.0}'::real[]`. Non-finite or negative weights are refused.

### Document forms

| Builder | Renders |
|---|---|
| `pg_fts::column(&SEARCH_VECTOR)` | `"docs"."search_vector"` |
| `pg_fts::to_tsvector("english", &BODY)` | `to_tsvector('english', "docs"."body")` |

### Query functions

| Method | SQL |
|---|---|
| `matches_tsquery` | `to_tsquery` |
| `matches_plainto_tsquery` | `plainto_tsquery` |
| `matches_websearch_to_tsquery` | `websearch_to_tsquery` |

### The fixture

Live Postgres (`TEST_DB_PORT=5434`). `search_vector` is `GENERATED … setweight(body,'A') || setweight(tags,'B')`:

| id | body | tags | deleted_at |
|---|---|---|---|
| m1 | the zebrafish keeps running fast | bio notes | null |
| m2 | a marathon runner trains daily | sport | null |
| m3 | shoes and laces | runner gear | null |
| m4 | completely unrelated text | none | 1 |

`@@ to_tsquery('english','runner')` → m2, m3 (m2 higher); A-only weights → m2; B-only → m3; `'run'` → m1 (stem); `websearch 'runner -shoes'` → m2 only.

### SQLite

Every constructor refuses at construction:

```
ts_rank is a Postgres full-text feature with no sqlite equivalent; build this query for
Postgres, or use the SQLite FTS5 surface (MATCH / bm25) instead
```

## How to run

```sh
cargo nextest run -p toolu-orm-core -E 'binary(pg_fts_query_test)'
cargo nextest run -p toolu-orm-query -E 'binary(pg_fts_select_sql_test)'
TEST_DB_PORT=5434 cargo nextest run -p toolu-orm-query --features postgres -E 'binary(pg_fts_postgres_test)'
```

## Tests

| Lane | Binary | Test |
|---|---|---|
| default | pg_fts_query_test | match_expr::column_tsquery_renders_and_binds_the_pattern |
| default | pg_fts_query_test | match_expr::tsquery_continues_the_callers_parameter_numbering |
| default | pg_fts_query_test | match_expr::to_tsvector_plainto_and_websearch_render |
| default | pg_fts_query_test | match_expr::match_composes_with_ordinary_filters |
| default | pg_fts_query_test | match_expr::the_short_forms_agree_with_the_current_dialect |
| default | pg_fts_query_test | rendering::ts_rank_without_weights_embeds_the_query |
| default | pg_fts_query_test | rendering::ts_rank_weights_render_first_as_real_array |
| default | pg_fts_query_test | rendering::a_quote_inside_the_rank_query_is_doubled |
| default | pg_fts_query_test | rendering::plainto_and_websearch_rank_variants_render |
| default | pg_fts_query_test | rendering::the_short_rank_forms_agree_with_the_current_dialect |
| default | pg_fts_query_test | rejections::sqlite_is_refused_by_every_constructor |
| default | pg_fts_query_test | rejections::an_invalid_config_is_refused |
| default | pg_fts_query_test | rejections::a_weight_that_is_not_finite_and_non_negative_is_refused |
| default | pg_fts_query_test | rejections::fts5_still_refuses_postgres_and_pg_fts_refuses_sqlite |
| default | pg_fts_select_sql_test | the_full_text_query_from_the_issue_is_expressible |
| default | pg_fts_select_sql_test | a_query_can_order_by_ts_rank_without_selecting_it |
| postgres | pg_fts_postgres_test | runner_ranks_m2_then_m3_and_excludes_soft_deleted |
| postgres | pg_fts_postgres_test | column_weights_change_which_row_wins |
| postgres | pg_fts_postgres_test | a_stemmed_term_matches_and_websearch_excludes |
