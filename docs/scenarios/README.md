# Scenarios

One page per feature scenario, each proven by tests that execute against real
databases: in-memory libsql, in-memory rusqlite, and a live Postgres. Like
drizzle, the goal is parity: a feature is done when it behaves the same on
SQLite and Postgres, and each page shows both.

These pages are kept in sync with the test suites by
`scripts/check-scenario-docs.sh` (run in CI and in the quality gate). It fails
when a test listed in a page's `## Tests` table does not exist, or when a test
in any binary named on these pages is missing from every page. Add or rename a
test, update its page.

| Scenario | What it proves |
|---|---|
| [Upsert](upsert.md) | `or_replace` / `or_ignore` on all three drivers (`INSERT OR REPLACE`, `ON CONFLICT ... DO UPDATE SET ... = EXCLUDED`). |
| [Relational loads](relational-loads.md) | `with_many` / `with_one` fetch parent and children in one statement, including empty and null relations and declared binary (`BLOB` / `bytea`) columns. |
| [Value round-trip](value-round-trip.md) | Every `Value` variant binds as a parameter and reads back unchanged. |
| [Fetch semantics](fetch-semantics.md) | `fetch_one` / `fetch_optional` / `count` / `exists` and their empty, single, and multi-row behavior. |
| [Bounded first-row fetch](bounded-first-row.md) | `fetch_one` / `fetch_optional` ask the database for at most one row, so a 10,000-row match costs one decode and a later undecodable row cannot fail a valid first row. |
| [SQLite offset without limit](sqlite-offset-without-limit.md) | `.offset(n)` without `.limit(...)` renders `LIMIT -1 OFFSET ...` on SQLite instead of an invalid bare `OFFSET`, with correct parameter numbering; Postgres's standalone `OFFSET` stays unaffected. |
| [Table aliases and JOIN predicates](table-aliases-and-joins.md) | `TableRef` aliases a table, `AliasedColumn` addresses its columns, `columns_qualified` / `column_as` project unambiguously, and an `ON` clause is an expression tree — self-joins, the same table joined twice, and a `LEFT JOIN` that keeps its unmatched rows. |
| [Filters](filters.md) | Every `Expr` operator executed against real rows, parameter numbering across filters, nested AND/OR, empty `in_list`. |
| [Transactions](transactions.md) | Commit persists; rollback and drop discard; reads inside see own writes (libsql `run_transaction`, orm-query `PgTransaction`, connection `PgTransaction`). |
| [Postgres connection](postgres-connection.md) | `PgDatabase` pool, `PgConnection`, error mapping with SQLSTATE, unreachable server, and the per-connection prepared-statement cache. |
| [Blocking connection](blocking-connection.md) | `DbConnectionBlocking` on rusqlite with no runtime: migrate/status/baseline twins, `Executor for RusqliteConnection`, non-`Send` rows, async agreement, contention, poisoning. |
| [FromRow derive](from-row-derive.md) | `#[derive(FromRow)]` on every driver shape against real rows: NULL to `None`, missing columns, `#[from_row(with)]`. |
| [Driver feature unification](driver-feature-unification.md) | Only orm-core picks a `FromRow` method; every other crate decodes through `row::from_{postgres,libsql,rusqlite}_row`, and all eight driver combinations compile. |
| [Migration loop](migration-loop.md) | generate → migrate → evolve → generate → migrate, asserted through `PRAGMA` / `information_schema`. |
| [SQLite table rebuild](sqlite-table-rebuild.md) | A column change SQLite cannot make in place rebuilds the table without cascade-deleting child rows, corrupting their foreign keys, or losing indexes. |
| [Migration baseline](migration-baseline.md) | `mark_applied` / `mark_applied_through` adopt an existing database by recording journal entries without running their SQL. |
| [Embedded migrations](embedded-migrations.md) | `run_migrate_embedded` applies an `include_str!`'d list with the same hashes and transactions as the directory runner, and the two sources interchange. |
| [Migration history integrity](migration-history-integrity.md) | Already-applied migrations are re-checked against the journal (or embedded list) and their current bytes before a run skips them, with defined behavior for unhashed rows and pruned histories. |
| [Migration failures](migration-failures.md) | Rollback after a failing statement, unreadable inputs, malformed journal, absent directory, comment-only chunks. |
| [Scalar expressions and LIKE ESCAPE](scalar-expressions.md) | `Scalar` columns, binds, function calls, arithmetic, `\|\|` and `CASE` composed into filters, projections, `ORDER BY`, INSERT values and UPDATE assignments, with one bind pipeline and an explicit `LIKE … ESCAPE`. |
| [DISTINCT, GROUP BY, HAVING and aggregates](distinct-and-grouping.md) | `distinct()`, `group_by` / `group_by_scalar`, `having`, and the `Scalar` aggregate constructors (`COUNT(*)`, `COUNT(DISTINCT …)`, `SUM`, `MAX`, `MIN`, `AVG`) executed per group — deduplication applied before pagination, bound `HAVING` parameters, empty result sets, and a grouped `count()` that reports the number of groups. |
| [INSERT … SELECT and database-qualified names](insert-select.md) | `InsertBuilder::select` fills an INSERT from a whole SELECT — one statement, no row decoded into Rust — and `TableRef::in_database` renders `main.indexed_files` as two identifiers, proven by a real `ATTACH`-based copy carrying NULLs, blobs and an old-schema projected default. |
| [Reusable bound parameters](reusable-bound-parameters.md) | `SharedBind` / `SharedBindList` bind a value or a list once and let every predicate reference the same placeholder — the issue's 16,381-path two-orientation lookup drops from 32,767 parameters (which SQLite refuses) to 16,385, with identity by handle rather than by value. |
| [Query composition](query-composition.md) | `WITH` / `WITH RECURSIVE` CTEs, `UNION` / `UNION ALL`, subqueries in predicate and scalar position, and table-valued `FROM` sources with bound arguments — a recursive walk that terminates on a cycle, NULL-sensitive `NOT EXISTS`, and a `DELETE … WHERE id IN (SELECT …)` whose id set never reaches Rust. |
| [Expression fragments](expr-fragments.md) | `Expr` SQL fragments with parameter offsets per dialect, nesting, and the empty-list constant. |
| [Virtual tables (FTS5)](virtual-tables.md) | `TableKind::Virtual` DDL, `#[fts5_table]`, real `MATCH` queries, and the refusal to alter one in place. |
| [FTS5 synchronization triggers](fts5-sync-triggers.md) | `sync_content()` puts the three triggers an external-content FTS5 index needs into the schema, so writes to the content table stay indexed and the triggers survive every recreation. |
| [FTS5 queries](fts5-queries.md) | `MATCH` in `Expr`, `bm25` / `rank` / `snippet` / `highlight` as selectable and orderable expressions, validated weights, and the Postgres refusal. |
| [Postgres FTS queries](postgres-fts-queries.md) | `@@` / `to_tsquery` / `ts_rank` as a Postgres-only builder surface, positive scores / `DESC`, and the SQLite refusal. |
| [vec0 virtual tables](vec0-virtual-tables.md) | `ColumnType::Vector`, `#[vec0_table]`, `Value::vector`, and `MigrateError::MissingExtension` when `sqlite-vec` is not loaded. |
| [vec0 KNN](vec0-knn.md) | `SelectBuilder::knn` (`MATCH` + hidden `k`), `vec0::distance`, Postgres refusal, and the filtered-KNN oversample note. |
| [pgvector KNN](pgvector-knn.md) | `<->` / `<=>` / `<#>` distance `ORDER BY … LIMIT k` as a Postgres-only surface, embedded vector literals, and the SQLite refusal. |
| [Legacy snapshot](legacy-snapshot.md) | Old snapshot JSON shapes still deserialize and diff. |
| [Partial indexes](partial-indexes.md) | `#[index(..., where = "…")]` predicates on `IndexDef`, SQL render, serde, and drop+create diffs. |
| [Index column DESC](index-desc.md) | `desc(col)` in `#[index]` / `#[unique_index]`, SQL `DESC`, serde string lists, diff drop+create. |
| [Composite primary key](composite-primary-key.md) | Table-level `#[primary_key(...)]`, `AUTOINCREMENT` / Postgres `IDENTITY`, and key-set recreation. |
| [Renames](renames.md) | A `RenameResolver` turns drop+create into `RENAME TABLE` / `RENAME COLUMN`. |
| [Macro compile errors](macro-compile-errors.md) | Each proc-macro error message pinned by trybuild. |
| [Column CHECK attribute](column-check.md) | `#[column(check = "...")]` sets `ColumnDef.check`; SQL and snapshot use the existing path. |
| [Facade crate](facade.md) | `toolu-orm` re-exports the stack; `#[table]` and the builders work with it as the only dependency. |
| [Lanes and revived suites](lanes.md) | Which CI lane compiles which suite, and the executor/transaction suites brought back from bit-rot. |
| [Prepared-statement cache](prepared-statement-cache.md) | Both rusqlite adapters reuse `Connection::prepare_cached` across changed parameters, a schema change, an error followed by reuse, and a row-mapping failure. |
| [rusqlite async backpressure](rusqlite-async-backpressure.md) | The async rusqlite path admits one operation at a time before `spawn_blocking`, so a contended connection cannot starve tokio's blocking pool, and cancellation on either side of admission is bounded. |
| [SQLite maintenance operations](sqlite-maintenance-ops.md) | `VACUUM INTO`, `PRAGMA quick_check`, quoted `ATTACH` with a `Drop`-guaranteed `DETACH`, and typed `page_count` / `page_size` on a borrowed `rusqlite::Connection` — plus the documented FFI exception for the FTS5 tokenizer handshake. |
