# toolu-orm-core

Foundation crate for toolu-orm — core types, schema definitions, snapshot/diff engine, and SQL generation.

## Crate Type
- Library
- Internal deps: none (foundation crate)
- Features: `libsql` (default), `rusqlite`, `postgres`; all eight combinations compile

## Crate-Specific Rules
- Enabled driver crates are re-exported as `libsql`, `rusqlite` and `tokio_postgres`, so consumers can name driver types without a separate direct dependency.
- `FromRow` uses `from_row` with one driver, driver-specific methods with multiple drivers, and no decoder with none. Its shape follows the features unified on orm-core; callers use the `row::from_*_row` helpers.
- `Value` bridges ORM values to libsql, rusqlite and Postgres parameters
- `Column<T>` uses PhantomData marker types for type-safe operations (CommonOps, TextOps, NumericOps)
- Snapshot serialization uses BTreeMap for deterministic ordering
- Migration hashes use SHA256 with `sha256:` prefix
- `Expr::to_sql_fragment(start)` uses parameter index offset for composable WHERE clauses

## Key Modules
- `column.rs` — ColumnType, ColumnDef, ForeignKeyAction, marker types
- `table.rs` — TableDef, TableSchema trait
- `policy.rs` — Postgres row-level security: `RowSecurity` (force flag + `PolicyDef` list) on `TableDef.row_security`; `diff/policy.rs` validates and diffs it, `sql/policy.rs` renders it (comments on SQLite)
- `schema.rs` — SchemaRegistry (collection of TableDefs)
- `value.rs` — Value enum with database driver conversions
- `expr/` — Expression AST: `types/`, `scalar/`, `render/`, and `binding/`. Renderers share `BoundParams` and take the next index from `params.next_index()`; they never add a separate offset. `nested(start, ...)` handles standalone fragments and nested statements. Shared handles reuse their recorded indices; empty `in_list` renders `1 = 0`.
- `alias/` — TableRef (table or bound table-valued function, optional database/schema and alias), AliasedColumn\<T\>, the object-safe QualifiedColumn trait, and six column-to-column comparisons
- `query_column/` — Column\<T\>, ColumnRef trait, CommonOps/TextOps (`like`, `like_escape`)/NumericOps traits, Fts5Ops, Vec0Ops
- `row/` — FromRow traits, driver decode helpers and `impl_derived_from_row!`
- `diff/` — Schema diff algorithm (Operation enum)
- `snapshot/` — Schema snapshot serialization
- `journal.rs` — Migration journal tracking
- `sql/` — SQL generation from Operation list, SQLite rebuilds and FTS5 sync triggers

## References
- Root CLAUDE.md (project-wide rules)
- Root `Cargo.toml`, `clippy.toml` and `scripts/check-file-length.sh`
