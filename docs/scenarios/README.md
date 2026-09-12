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
| [Relational loads](relational-loads.md) | `with_many` / `with_one` fetch parent and children in one statement, including empty and null relations. |
| [Value round-trip](value-round-trip.md) | Every `Value` variant binds as a parameter and reads back unchanged. |
| [Fetch semantics](fetch-semantics.md) | `fetch_one` / `fetch_optional` / `count` / `exists` and their empty, single, and multi-row behavior. |
| [Filters](filters.md) | Every `Expr` operator executed against real rows, parameter numbering across filters, nested AND/OR, empty `in_list`. |
| [Transactions](transactions.md) | Commit persists; rollback and drop discard; reads inside see own writes (libsql `run_transaction`, orm-query `PgTransaction`, connection `PgTransaction`). |
| [Postgres connection](postgres-connection.md) | `PgDatabase` pool, `PgConnection`, error mapping with SQLSTATE, unreachable server. |
| [Blocking connection](blocking-connection.md) | `DbConnectionBlocking` on rusqlite with no runtime: migrate/status/baseline twins, `Executor for RusqliteConnection`, non-`Send` rows, async agreement, contention, poisoning. |
| [FromRow derive](from-row-derive.md) | `#[derive(FromRow)]` on every driver shape against real rows: NULL to `None`, missing columns, `#[from_row(with)]`. |
| [Migration loop](migration-loop.md) | generate → migrate → evolve → generate → migrate, asserted through `PRAGMA` / `information_schema`. |
| [Migration baseline](migration-baseline.md) | `mark_applied` / `mark_applied_through` adopt an existing database by recording journal entries without running their SQL. |
| [Embedded migrations](embedded-migrations.md) | `run_migrate_embedded` applies an `include_str!`'d list with the same hashes and transactions as the directory runner, and the two sources interchange. |
| [Migration failures](migration-failures.md) | Rollback after a failing statement, unreadable inputs, malformed journal, absent directory, comment-only chunks. |
| [Expression fragments](expr-fragments.md) | `Expr` SQL fragments with parameter offsets per dialect, nesting, and the empty-list constant. |
| [Virtual tables (FTS5)](virtual-tables.md) | `TableKind::Virtual` DDL, `#[fts5_table]`, real `MATCH` queries, and the refusal to alter one in place. |
| [FTS5 queries](fts5-queries.md) | `MATCH` in `Expr`, `bm25` / `rank` / `snippet` / `highlight` as selectable and orderable expressions, validated weights, and the Postgres refusal. |
| [Postgres FTS queries](postgres-fts-queries.md) | `@@` / `to_tsquery` / `ts_rank` as a Postgres-only builder surface, positive scores / `DESC`, and the SQLite refusal. |
| [vec0 virtual tables](vec0-virtual-tables.md) | `ColumnType::Vector`, `#[vec0_table]`, `Value::vector`, and `MigrateError::MissingExtension` when `sqlite-vec` is not loaded. |
| [vec0 KNN](vec0-knn.md) | `SelectBuilder::knn` (`MATCH` + hidden `k`), `vec0::distance`, Postgres refusal, and the filtered-KNN oversample note. |
| [pgvector KNN](pgvector-knn.md) | `<->` / `<=>` / `<#>` distance `ORDER BY … LIMIT k` as a Postgres-only surface, embedded vector literals, and the SQLite refusal. |
| [Legacy snapshot](legacy-snapshot.md) | Old snapshot JSON shapes still deserialize and diff. |
| [Partial indexes](partial-indexes.md) | `#[index(..., where = "…")]` predicates on `IndexDef`, SQL render, serde, and drop+create diffs. |
| [Renames](renames.md) | A `RenameResolver` turns drop+create into `RENAME TABLE` / `RENAME COLUMN`. |
| [Macro compile errors](macro-compile-errors.md) | Each proc-macro error message pinned by trybuild. |
| [Column CHECK attribute](column-check.md) | `#[column(check = "...")]` sets `ColumnDef.check`; SQL and snapshot use the existing path. |
| [Facade crate](facade.md) | `toolu-orm` re-exports the stack; `#[table]` and the builders work with it as the only dependency. |
| [Lanes and revived suites](lanes.md) | Which CI lane compiles which suite, and the executor/transaction suites brought back from bit-rot. |
