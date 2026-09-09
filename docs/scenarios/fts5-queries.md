# FTS5 queries (MATCH, bm25, snippet)

**Feature:** `Expr::table_match` and `Fts5Ops::matches` put the `MATCH` operator in `filter(...)`; `fts5::bm25` / `rank` / `snippet` / `highlight` render the auxiliary functions as `Fts5Fn`, usable through `SelectBuilder::column_expr` as a named output and through `order_by`. [Virtual tables (FTS5)](virtual-tables.md) declares the index this reads.
**Drivers:** libsql and rusqlite (SQLite only). On Postgres every constructor refuses.
**Spec:** [FTS5 query surface](../toolu/specs/2026-09-07-fts5-query-surface-design.md), AC-1 … AC-10.

## What is proven

### The query that motivated this

Before this change, `Expr` covered `eq` / `in_list` / `is_null` / `like` / `between` and none of it reached an FTS5 index, so a full-text query dropped to hand-written SQL — taking its joins and its filters with it. It is now built end to end:

```rust
let score = fts5::bm25_for(Dialect::Sqlite, "memory_fts", &[0.0, 3.0, 1.0])?;

let hits: Vec<ScoredHit> = SelectBuilder::new("memory_fts")
  .columns_raw(&["memory_id"])
  .column_expr(score.sql(), "score")
  .join("memories", MEMORY_ID.equals(&FTS_MEMORY_ID))
  .filter(Expr::table_match_for(Dialect::Sqlite, "memory_fts", "runner")?)
  .filter(MEMORY_DELETED_AT.is_null())
  .order_by(OrderBy::alias_asc("score"))
  .limit(10)
  .fetch_all(&conn)
  .await?;
```

```sql
SELECT "memory_id", bm25("memory_fts", 0.0, 3.0, 1.0) AS "score"
  FROM "memory_fts"
  INNER JOIN "memories" ON "memories"."id" = "memory_fts"."memory_id"
 WHERE "memory_fts" MATCH ?1 AND "memories"."deleted_at" IS NULL
 ORDER BY "score" ASC LIMIT ?2
```

The `MATCH` pattern is bound (`?1`), shares one parameter sequence with the ordinary filters, and the whole statement contains no `format!` written by the caller.

### `bm25` is negative, so `ASC` is best first

FTS5 relevance scores are **negative**, and a better match is **more** negative. `ORDER BY score ASC` therefore returns the best match first, which inverts everyone's intuition once. `rank` is the same score under default weights and orders the same way. Both are asserted against a real index rather than described here only.

### Weights are literals, and they are validated

FTS5 rejects bound parameters as auxiliary-function arguments — `bm25(t, ?, ?)` is a syntax error — so weights must be formatted into the SQL text. `bm25` does that once, under validation, instead of every consumer doing it in a `format!`:

| Weights | Renders | Effect on the fixture below |
|---|---|---|
| `&[0.0, 3.0, 1.0]` | `bm25("memory_fts", 0.0, 3.0, 1.0)` | `m2` first — the term is in its `body` |
| `&[0.0, 1.0, 3.0]` | `bm25("memory_fts", 0.0, 1.0, 3.0)` | `m3` first — the term is in its `tags` |
| `&[]` | `bm25("memory_fts")` | FTS5's default, 1.0 everywhere |

`0.0` must render `0.0` and not `0`: Display would emit an integer literal, so the renderer uses `{:?}`. The rusqlite suite proves it behaviourally — a zero-weighted column stops contributing and the other row wins.

A weight is refused when it is `NaN`, infinite, or **negative**. Negative is not a matter of taste: `bm25(t, -1.0, 1.0)` returns a *positive* score on a real build, silently inverting the ordering everything else here depends on.

### Column-qualified versus table-level MATCH

`"memory_fts" MATCH ?` searches every indexed column and is the common case; `"memory_fts"."body" MATCH ?` narrows to one. Against the fixture, `body MATCH 'runner'` returns only `m2` and `tags MATCH 'runner'` only `m3`. An `UNINDEXED` column is still not searchable from either form, which is the read side agreeing with the write side.

### The fixture

Declared through `Fts5Table` and created by the real DDL generator, so the table these queries read is the table [Virtual tables (FTS5)](virtual-tables.md) creates. `memory_fts(memory_id UNINDEXED, body, tags)`, `tokenize = 'porter unicode61 remove_diacritics 2'`:

| id | body | tags | `memories.deleted_at` |
|---|---|---|---|
| m1 | the zebrafish keeps running fast | bio notes | null |
| m2 | a marathon runner trains daily | sport | null |
| m3 | shoes and laces | runner gear | null |
| m4 | completely unrelated text | none | 1 |

`MATCH 'runner'` → `m2, m3`; `MATCH 'run'` → `m1` (the porter tokenizer really is in force); `MATCH 'm1'` → nothing; `MATCH 'kangaroo'` → nothing. `snippet` over `body` returns `a marathon <b>runner</b> trains daily`; `highlight` over `tags` returns `<b>runner</b> gear`.

### Postgres

`MATCH` and the FTS5 auxiliary functions are SQLite's. Postgres full-text is `@@` / `to_tsquery` / `ts_rank` — use the typed [Postgres FTS queries](postgres-fts-queries.md) surface instead of translating silently. Every FTS5 constructor still refuses on Postgres:

```
bm25 is a SQLite FTS5 feature with no postgres equivalent; build this query for SQLite,
or write the Postgres full-text form (@@ / to_tsquery / ts_rank) yourself
```

The refusal happens where the expression is **constructed**, not where it is rendered. That is deliberate and stronger than rejecting at render time: a Postgres-bound `Expr::Match` or `Fts5Fn` never exists, so no statement can be generated carrying one, and the existing `to_sql_for` / `to_sql_fragment_for` signatures stay infallible for the rest of the builder. The short forms (`fts5::bm25`, `Expr::table_match`, `Column::matches`) follow `Dialect::CURRENT` and so refuse on their own in a postgres build; the `_for` forms take the dialect explicitly.

### Everything else that is refused

| Argument | Refused because |
|---|---|
| table name empty, or outside `[A-Za-z_][A-Za-z0-9_]*` | it addresses an FTS5 table; a name is validated, not escaped, so no quote, `;` or `--` is ever emitted |
| a table *alias* | FTS5 does not accept one (`bm25(f, …)` → `no such column: f`) |
| `snippet` tokens outside `1..=64` | FTS5 documents that range but silently clamps, so refusing is the only way the caller learns |
| `column_index` below `-1` | `-1` means every column and `0..` names one; below that is meaningless |
| an unknown name in `fts5::column_index` | the error lists the columns that do exist |

A `column_index` past the table's last column cannot be caught here — the column count is unknown at that point — and surfaces as a driver error when the statement is prepared.

## How to run

```sh
cargo nextest run -p toolu-orm-core -E 'binary(fts5_query_test)'
cargo nextest run -p toolu-orm-query -E 'binary(fts5_select_sql_test)'
cargo nextest run -p toolu-orm-query --features libsql -E 'binary(fts5_libsql_test)'
cargo nextest run -p toolu-orm-query --features rusqlite -E 'binary(fts5_rusqlite_test)'
```

## Tests

| Lane | Binary | Test |
|---|---|---|
| default | fts5_query_test | match_expr::table_match_renders_the_quoted_table_and_binds_the_pattern |
| default | fts5_query_test | match_expr::table_match_continues_the_callers_parameter_numbering |
| default | fts5_query_test | match_expr::column_match_qualifies_the_column |
| default | fts5_query_test | match_expr::match_composes_with_the_ordinary_operators_and_shares_their_numbering |
| default | fts5_query_test | match_expr::the_short_match_forms_agree_with_the_current_dialect |
| default | fts5_query_test | rendering::bm25_renders_every_weight_as_a_float_literal |
| default | fts5_query_test | rendering::bm25_without_weights_omits_the_argument_list |
| default | fts5_query_test | rendering::rank_is_the_bare_column_not_a_call |
| default | fts5_query_test | rendering::snippet_and_highlight_render_their_arguments_in_order |
| default | fts5_query_test | rendering::a_quote_inside_a_tag_is_doubled |
| default | fts5_query_test | rendering::a_call_orders_ascending_and_descending |
| default | fts5_query_test | rendering::an_alias_orders_by_the_computed_output |
| default | fts5_query_test | rendering::column_index_finds_the_column_by_declaration_order |
| default | fts5_query_test | rendering::the_short_forms_agree_with_the_current_dialect |
| default | fts5_query_test | rejections::postgres_is_refused_by_every_auxiliary_function |
| default | fts5_query_test | rejections::postgres_is_refused_by_both_match_forms |
| default | fts5_query_test | rejections::a_weight_that_is_not_finite_and_non_negative_is_refused |
| default | fts5_query_test | rejections::a_table_name_that_is_not_a_plain_identifier_is_refused |
| default | fts5_query_test | rejections::a_token_count_outside_the_fts5_range_is_refused |
| default | fts5_query_test | rejections::a_column_index_below_minus_one_is_refused_while_minus_one_means_every_column |
| default | fts5_query_test | rejections::an_unknown_column_name_is_refused_and_names_the_columns_that_exist |
| default | fts5_select_sql_test | the_full_text_query_from_the_issue_is_expressible |
| default | fts5_select_sql_test | a_query_can_order_by_the_score_without_selecting_it |
| default | fts5_select_sql_test | a_non_raw_builder_keeps_a_column_expr_alongside_its_columns |
| default | fts5_select_sql_test | a_column_expr_without_columns_renders_no_leading_comma |
| default | fts5_select_sql_test | count_and_exists_ignore_the_projection_but_keep_the_match |
| libsql-only | fts5_libsql_test | the_issue_query_runs_and_ranks_against_a_real_index |
| libsql-only | fts5_libsql_test | bm25_scores_are_negative_and_ascending_order_is_best_first |
| libsql-only | fts5_libsql_test | rank_orders_the_same_way_as_bm25 |
| libsql-only | fts5_libsql_test | a_stemmed_term_matches_and_a_missing_one_returns_nothing |
| libsql-only | fts5_libsql_test | the_unindexed_column_is_not_searchable_and_columns_narrow_the_match |
| libsql-only | fts5_libsql_test | count_and_exists_answer_a_match_filter |
| rusqlite-only | fts5_rusqlite_test | column_weights_change_the_returned_order |
| rusqlite-only | fts5_rusqlite_test | a_zero_weight_excludes_its_column_from_the_score |
| rusqlite-only | fts5_rusqlite_test | the_issue_query_runs_and_excludes_the_soft_deleted_row |
| rusqlite-only | fts5_rusqlite_test | snippet_marks_up_the_matched_term |
| rusqlite-only | fts5_rusqlite_test | highlight_marks_up_the_whole_column |
| rusqlite-only | fts5_rusqlite_test | the_unindexed_column_is_not_searchable |
