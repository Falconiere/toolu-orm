# toolu-orm-query

Type-safe query builders for toolu-orm — Select, Insert, Update, Delete with feature-gated executors.

## Crate Type
- Library
- Internal deps: toolu-orm-core; toolu-orm-connection under `rusqlite`; sqlite-vec register helper under `sqlite-vec`
- No default driver. Features: `libsql` (async), `rusqlite` (sync), `postgres` (async), and dependency-only `lancedb`; execution requires exactly one implemented driver on this crate with `lancedb` absent

## Crate-Specific Rules
- The libsql/rusqlite scalar decoders require the matching single-driver shape on orm-core; do not enable extra core drivers behind a single-driver SQLite query build.
- Builders are database-agnostic — explicit `to_sql_for(Dialect::Lance)` renders `?N` without a driver, including under mixed `postgres,lancedb` features; `to_sql()` retains compile-time `Dialect::CURRENT`
- `execute()` methods are feature-gated: async for libsql/Postgres, sync for rusqlite
- `Executor` trait provides `dialect()`, `execute_sql()`, and `query_map()` — built-in implementations report SQLite/Postgres; builder execution renders for `exec.dialect()` while the compatibility default is `CURRENT`
- No production Lance executor exists in this crate; issue #174 owns refusal of Lance `ON CONFLICT` and ordinary DML `RETURNING` before execution
- `impl_filter!` and `impl_execute!` macros reduce boilerplate across builders
- `SelectBuilder` supports raw mode (`SelectBuilder::raw()`) for custom queries
- Every clause renders through one `BoundParams` in SQL order: WITH, select list, FROM/JOIN arguments and ON, WHERE, GROUP BY/HAVING, compound arms, ORDER BY, LIMIT/OFFSET. UPDATE binds SET before WHERE; INSERT binds VALUES or the SELECT source before ON CONFLICT. Take `params.next_index()` rather than adding an offset; the shared ledger also spans nested statements.

## Key Modules
- `select/builder.rs` — SelectBuilder (columns, joins, filters, limit, offset)
- `select/projection.rs` — `column_expr` (raw text) / `column_scalar` (a `Scalar`, which may bind) and the select list they render into
- `select/ordering.rs` — `order_by` and the ORDER BY tail; a term may bind, so it renders after WHERE and before LIMIT
- `select/row_limit.rs` — the LIMIT/OFFSET tail, and `to_first_row_sql` (the at-most-one-row query `fetch_one` / `fetch_optional` send)
- `select/{cte,compound,source,grouping,count_exists}.rs` — CTEs, UNION, subquery integration, grouping and aggregate queries
- `insert/` — InsertBuilder (values or SELECT source, explicit OnConflict, legacy conflict modes, RETURNING and fetch methods)
- `update.rs` — UpdateBuilder (set, set_expr, set_scalar, filter)
- `delete.rs` — DeleteBuilder (filter)
- `executor/` — Executor implementations for libsql, rusqlite (raw and wrapped), Postgres Client and the query PgTransaction
- `transaction.rs` — Transaction wrapper (commit, rollback)
- `where_clause.rs` — Filter macro and WHERE clause generation
- `error.rs` — QueryError (Driver, RowMapping, NotFound, Transaction)

## References
- Root CLAUDE.md (project-wide rules)
- Root `Cargo.toml`, `clippy.toml` and `scripts/check-file-length.sh`
