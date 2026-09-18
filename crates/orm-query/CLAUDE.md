# toolu-orm-query

Type-safe query builders for toolu-orm — Select, Insert, Update, Delete with feature-gated executors.

## Crate Type
- Library
- Internal deps: toolu-orm-core
- Features: `libsql` (default, async), `rusqlite` (sync)

## Crate-Specific Rules
- Builders are database-agnostic — `to_sql()` works without any feature flag
- `execute()` methods are feature-gated: async for libsql, sync for rusqlite
- `Executor` trait provides `execute_sql()` and `query_map()` — implementations differ per driver
- `impl_filter!` and `impl_execute!` macros reduce boilerplate across builders
- `SelectBuilder` supports raw mode (`SelectBuilder::raw()`) for custom queries
- WHERE clause parameter indexing starts from `start` offset for composability
- Every clause renders into one `params` vec in statement order — SelectBuilder: select list, WHERE, ORDER BY, LIMIT/OFFSET; UpdateBuilder: SET then WHERE; InsertBuilder: the VALUES list in column order — so a `Scalar` can bind in any of them and the numbering stays correct

## Key Modules
- `select/builder.rs` — SelectBuilder (columns, joins, filters, limit, offset)
- `select/projection.rs` — `column_expr` (raw text) / `column_scalar` (a `Scalar`, which may bind) and the select list they render into
- `select/ordering.rs` — `order_by` and the ORDER BY tail; a term may bind, so it renders after WHERE and before LIMIT
- `select/row_limit.rs` — the LIMIT/OFFSET tail, and `to_first_row_sql` (the at-most-one-row query `fetch_one` / `fetch_optional` send)
- `insert.rs` — InsertBuilder (set, set_null, set_scalar, or_replace, or_ignore)
- `update.rs` — UpdateBuilder (set, set_expr, set_scalar, filter)
- `delete.rs` — DeleteBuilder (filter)
- `executor.rs` — Executor trait + libsql/rusqlite implementations
- `transaction.rs` — Transaction wrapper (commit, rollback)
- `where_clause.rs` — Filter macro and WHERE clause generation
- `error.rs` — QueryError (Driver, RowMapping, NotFound, Transaction)

## References
- Root CLAUDE.md (project-wide rules)
- `docs/rules/forbidden-syntax-rust.md`
