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

## Key Modules
- `select/builder.rs` — SelectBuilder (columns, joins, filters, order, limit, offset)
- `insert.rs` — InsertBuilder (set, or_replace, or_ignore)
- `update.rs` — UpdateBuilder (set, set_expr, filter)
- `delete.rs` — DeleteBuilder (filter)
- `executor.rs` — Executor trait + libsql/rusqlite implementations
- `transaction.rs` — Transaction wrapper (commit, rollback)
- `where_clause.rs` — Filter macro and WHERE clause generation
- `error.rs` — QueryError (Driver, RowMapping, NotFound, Transaction)

## References
- Root CLAUDE.md (project-wide rules)
- `docs/rules/forbidden-syntax-rust.md`
