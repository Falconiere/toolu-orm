# Rust DuckDB–Lance SELECT and DML capability matrix

**Scope:** Issue #156 measures SQL emitted by the existing ORM builders against real local Lance tables through the pinned Rust `duckdb` binding. The pinned dependency pair, artifact hashes, and missing-extension behavior are recorded in the [Rust smoke probe](lancedb-rust-smoke.md). This page is evidence for future renderer and capability work, not a shipped Lance query adapter.

The probe renders the existing builders with `Dialect::Lance` and binds the returned `Value` sequence to a prepared DuckDB statement. The fixture uses `items(id BIGINT, group_id BIGINT, name VARCHAR, score BIGINT)` with `(1,1,one,10)`, `(2,1,two,20)`, `(3,2,three,30)`, `(4,99,orphan,40)`, plus `groups(1,alpha)` and `groups(2,beta)`. Each test creates independent disposable Lance datasets. Only the fixture's integer and text `Value` variants are converted in this probe; issue #148 owns the production scalar codec.

## Observed SELECT forms

The rows below use the builder's Lance rendering with `?N` placeholders; the values column follows parameter order. The parity links name existing real SQLite and PostgreSQL suites that run in the full repository gate. They prove the same SQL forms on those suites' own fixtures; same-seed three-backend acceptance belongs to #179. Issue #155 supplies the selected Lance renderer; issues #149/#169–#172 execute the individual surfaces.

| Form | SQL | Bound values | Observed result | SQLite/PostgreSQL parity | Required later change |
|---|---|---|---|---|---|
| Filter, projection, order, page | `SELECT "items"."id", "items"."name" FROM "items" WHERE "items"."score" > ?1 ORDER BY "items"."id" ASC LIMIT ?2 OFFSET ?3` | `15, 2, 1` | `(3,three), (4,orphan)` after skipping row 2. | [Filters](filters.md), [fetch semantics](fetch-semantics.md) | No syntax change; #149 runs this form through #162. |
| Offset without limit | `SELECT "items"."id" FROM "items" ORDER BY "items"."id" ASC OFFSET ?1` | `2` | IDs `3, 4`; no SQLite-only `LIMIT -1` prefix. | [SQLite offset form](sqlite-offset-without-limit.md) | #149 runs this form through #162. |
| Current Unix epoch expression | `SELECT floor(epoch(now()))::bigint AS "epoch"` | None | One integer within the observed system-clock interval. | [Scalar expressions](scalar-expressions.md) | #149 can use the dialect expression in SELECT projections. |
| INNER JOIN | `SELECT "items"."id", "groups"."label" AS "label" FROM "items" INNER JOIN "groups" ON "groups"."id" = "items"."group_id" WHERE "groups"."label" = ?1 ORDER BY "items"."id" ASC` | `alpha` | `(1,alpha), (2,alpha)` | [Table aliases and joins](table-aliases-and-joins.md) | No syntax change; #169 executes this builder form. |
| LEFT JOIN with unmatched right side | `SELECT "items"."id", "groups"."label" AS "label" FROM "items" LEFT JOIN "groups" ON "groups"."id" = "items"."group_id" WHERE "items"."id" = ?1` | `4` | `(4,NULL)` | [Table aliases and joins](table-aliases-and-joins.md) | No syntax change; #169 must preserve the null right side. |
| DISTINCT | `SELECT DISTINCT "items"."group_id" FROM "items" WHERE "items"."score" > ?1 ORDER BY "items"."group_id" ASC` | `15` | `1, 2, 99` | [Distinct and grouping](distinct-and-grouping.md) | No syntax change; #163 executes this form. |
| GROUP BY and HAVING | `SELECT "items"."group_id", COUNT(*) AS "n" FROM "items" WHERE "items"."score" > ?1 GROUP BY "items"."group_id" HAVING COUNT(*) > ?2` | `5, 1` | `(1,2)` | [Distinct and grouping](distinct-and-grouping.md) | No syntax change; #170 executes this form. |
| CTE | `WITH "hits" AS (SELECT "items"."id" FROM "items" WHERE "items"."score" > ?1) SELECT "id" FROM "hits" WHERE "hits"."id" < ?2 ORDER BY "id" ASC` | `15, 4` | `2, 3` | [Query composition](query-composition.md) | No syntax change; #171 executes this form. |
| Nested `IN` subquery | `SELECT "items"."id" FROM "items" WHERE "items"."score" > ?1 AND "items"."id" IN (SELECT "items"."id" FROM "items" WHERE "items"."group_id" = ?2)` | `20, 2` | `3` | [Query composition](query-composition.md) | No syntax change; #171 preserves bind numbering across the nested statement. |
| UNION | `SELECT "items"."id" FROM "items" WHERE "items"."group_id" = ?1 UNION SELECT "items"."id" FROM "items" WHERE "items"."score" > ?2 ORDER BY "id" ASC` | `1, 15` | `1, 2, 3, 4`; row 2 appears in both arms and is returned once. | [Query composition](query-composition.md) | No syntax change; #172 executes this form. |

## Observed DML forms

The same pinned run executed every row below with prepared values. Plain writes and raw MERGE changed the expected rows; `ON CONFLICT` and ordinary `RETURNING` failed with the exact errors shown and left the before image unchanged. The Lance fixture has no unique or primary-key constraint. SQLite/PostgreSQL upsert parity tests use their own unique-key fixtures, so their success does not imply that guarantee on Lance. Issue #174 should reject `ON CONFLICT` and ordinary DML `RETURNING` before a Lance write; #177 owns a portable MERGE builder and its key semantics.

| Form | SQL | Bound values | Observed result or exact error | SQLite/PostgreSQL parity | Required later change |
|---|---|---|---|---|---|
| INSERT builder | `INSERT INTO "items" ("id", "group_id", "name", "score") VALUES (?1, ?2, ?3, ?4)` | `5, 2, five, 50` | One row added: `(5,FIVE,50)` after the following UPDATE. | [Upsert and mutations](upsert.md) | No syntax change; #150/#173 execute with production Lance value binds from #148. |
| INSERT … SELECT builder | `INSERT INTO "items" ("group_id", "name", "score", "id") SELECT "items"."group_id", "items"."name", "items"."score", ?1 AS "id" FROM "items" WHERE "items"."id" = ?2` | `5, 1` | Copied seeded row 1 into `(5,1,one,10)`; one row inserted. | [INSERT … SELECT](insert-select.md) | #150/#173 execute this form with production Lance value binds from #148. |
| UPDATE builder | `UPDATE "items" SET "name" = ?1 WHERE "items"."id" = ?2` | `FIVE, 5`; missing-key repeat `missing, 999` | One row changed to `FIVE`; missing key changed zero rows. | [Upsert and mutations](upsert.md) | No syntax change; #175/#173 execute this form. |
| DELETE builder | `DELETE FROM "items" WHERE "items"."id" = ?1` | `5`; missing-key repeat `999` | One row removed; missing key changed zero rows; four seeded rows remain. | [Upsert and mutations](upsert.md) | No syntax change; #176/#173 execute this form. |
| MERGE engine control, raw SQL because no ORM builder exists | `MERGE INTO items AS target USING (SELECT ?1 AS id, ?2 AS name) AS incoming ON target.id = incoming.id WHEN MATCHED THEN UPDATE SET name = incoming.name WHEN NOT MATCHED THEN INSERT (id, group_id, name, score) VALUES (incoming.id, 2, incoming.name, 0)` | `2, merged`; then `5, new` | First call updated row 2; second inserted row 5; final rows include `(2,merged,20)` and `(5,new,0)`. | No existing builder parity; #177 will define and test it. | #177 must add an explicit MERGE builder/contract; this probe makes no uniqueness claim. |
| ON CONFLICT builder | `INSERT INTO "items" ("id", "group_id", "name", "score") VALUES (?1, ?2, ?3, ?4) ON CONFLICT ("id") DO NOTHING` | `1, 1, replacement, 100` | `Binder Error: The specified columns as conflict target are not referenced by a UNIQUE/PRIMARY KEY CONSTRAINT or INDEX`; rows unchanged. | [Upsert](upsert.md) succeeds on SQLite/PostgreSQL with unique-key fixtures. | #174 must return an unsupported-capability error before executing this Lance form. |
| INSERT RETURNING builder | `INSERT INTO "items" ("id", "group_id", "name", "score") VALUES (?1, ?2, ?3, ?4) RETURNING "id"` | `5, 2, five, 50` | `Not implemented Error: Lance INSERT does not support RETURNING yet`; rows unchanged. | [Upsert](upsert.md) returns the inserted ID on SQLite/PostgreSQL. | #174 must reject ordinary INSERT RETURNING before execution. |
| UPDATE RETURNING builder | `UPDATE "items" SET "name" = ?1 WHERE "items"."id" = ?2 RETURNING "id"` | `changed, 1` | `Not implemented Error: Lance UPDATE does not support RETURNING`; rows unchanged. | [Mutation parity](mutation-parity.md) returns the changed row on SQLite/PostgreSQL. | #174 must reject ordinary UPDATE RETURNING before execution. |
| DELETE RETURNING builder | `DELETE FROM "items" WHERE "items"."id" = ?1 RETURNING "id"` | `1` | `Not implemented Error: Lance DELETE does not support RETURNING yet`; rows unchanged. | [Mutation parity](mutation-parity.md) returns the removed row on SQLite/PostgreSQL. | #174 must reject ordinary DELETE RETURNING before execution. |

## Run

```sh
bash scripts/check-lancedb-smoke.sh
```

The runner downloads and verifies the pinned Lance extension, checks formatting and Clippy, confirms the documented test names, and executes the real Rust probe. Existing live SQLite and PostgreSQL scenario suites run in the repository's full quality gate.

## Tests

| Lane | Binary | Test |
|---|---|---|
| lancedb-matrix | lancedb_select_matrix_test | bound_filter_projection_order_and_page_return_expected_rows |
| lancedb-matrix | lancedb_select_matrix_test | offset_without_limit_uses_lance_syntax |
| lancedb-matrix | lancedb_select_matrix_test | now_epoch_expression_matches_the_system_clock |
| lancedb-matrix | lancedb_select_matrix_test | inner_and_left_join_preserve_matches_and_unmatched_rows |
| lancedb-matrix | lancedb_select_matrix_test | distinct_group_by_and_having_apply_bound_filters |
| lancedb-matrix | lancedb_select_matrix_test | cte_subquery_and_union_keep_bind_order_across_selects |
| lancedb-matrix | lancedb_dml_matrix_test | bound_insert_update_delete_change_only_matching_rows |
| lancedb-matrix | lancedb_dml_matrix_test | bound_insert_select_copies_a_seeded_row_with_source_binds |
| lancedb-matrix | lancedb_dml_matrix_test | bound_raw_merge_control_updates_and_inserts |
| lancedb-matrix | lancedb_dml_matrix_test | on_conflict_is_rejected_before_a_write |
| lancedb-matrix | lancedb_dml_matrix_test | ordinary_dml_returning_is_rejected_before_each_write |
