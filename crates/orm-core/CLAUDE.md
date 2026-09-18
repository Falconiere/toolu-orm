# toolu-orm-core

Foundation crate for toolu-orm — core types, schema definitions, snapshot/diff engine, and SQL generation.

## Crate Type
- Library
- Internal deps: none (foundation crate)
- Features: `libsql` (default), `rusqlite` (mutually exclusive)

## Crate-Specific Rules
- Dual database: `FromRow` trait definition changes based on active feature flag
- `Value` enum bridges ORM ↔ database driver types (libsql::Value or rusqlite types)
- `Column<T>` uses PhantomData marker types for type-safe operations (CommonOps, TextOps, NumericOps)
- Snapshot serialization uses BTreeMap for deterministic ordering
- Migration hashes use SHA256 with `sha256:` prefix
- `Expr::to_sql_fragment(start)` uses parameter index offset for composable WHERE clauses

## Key Modules
- `column.rs` — ColumnType, ColumnDef, ForeignKeyAction, marker types
- `table.rs` — TableDef, TableSchema trait
- `schema.rs` — SchemaRegistry (collection of TableDefs)
- `value.rs` — Value enum with database driver conversions
- `expr/` — Expression AST. `types/` (Expr + ExprKind, OrderBy, JoinCondition, JsonExpr), `scalar/` (Scalar + ScalarKind, function calls, arithmetic, `CASE`, `like_pattern_literal`), `render/` (per-dialect SQL rendering; empty `in_list` renders `1 = 0`, every node numbers its placeholders from `start + params.len()`)
- `query_column/` — Column\<T\>, ColumnRef trait, CommonOps/TextOps (`like`, `like_escape`)/NumericOps traits, Fts5Ops, Vec0Ops
- `row.rs` — FromRow trait (feature-gated)
- `diff.rs` — Schema diff algorithm (Operation enum)
- `snapshot.rs` — Schema snapshot serialization
- `journal.rs` — Migration journal tracking
- `sql.rs` — SQL generation from Operation list

## References
- Root CLAUDE.md (project-wide rules)
- `docs/rules/forbidden-syntax-rust.md`
