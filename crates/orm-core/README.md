# toolu-orm-core

Core types and schema engine for toolu-orm. Provides column types, table definitions, value conversions, type-safe expression building, schema snapshotting, diffing, and SQL generation.

## Stack

- **Serialization:** serde + serde_json (schema snapshots, journals)
- **Hashing:** sha2 (migration integrity)
- **Database:** libsql, rusqlite and Postgres (feature-gated, optional)

`libsql` is enabled by default. Disable default features when selecting another
driver or using only schema/SQL generation. Multiple drivers can coexist on core;
`FromRow` uses `from_row` with one driver and a method per driver with several.
With no drivers it carries only `REQUIRED_COLUMNS`.

## Architecture

```
src/
├── lib.rs              # Module exports
├── column.rs           # ColumnType, ColumnDef, marker types
├── table.rs            # TableDef, TableSchema trait
├── schema.rs           # SchemaRegistry
├── index.rs            # IndexDef
├── value.rs            # Value enum + driver conversions
├── row/                # FromRow traits, derives' dispatch macro and driver decode helpers
├── expr/               # Expr + Scalar trees (types/, scalar/) and per-dialect rendering (render/)
├── alias/              # TableRef, AliasedColumn<T>, QualifiedColumn, column-to-column comparisons
├── query_column/       # Column<T>, ColumnRef, CommonOps/TextOps/NumericOps/Fts5Ops/Vec0Ops
├── fts5/               # FTS5 table metadata, synchronization and expressions
├── vec0/               # SQLite vector-table metadata and expressions
├── pg_fts/             # Postgres full-text search expressions
├── pgvector/           # Postgres vector expressions
├── relation.rs         # Relation metadata
├── relational_row.rs   # Relational JSON row decoding
├── dialect.rs          # Dialect and placeholder/default translation
├── error.rs            # DbCoreError
├── diff/               # Fallible schema diff → Operation list
├── journal.rs          # Migration journal (entries + SHA256 hashes)
├── snapshot/           # Schema snapshot serialization
└── sql/                # Dialect-specific SQL generation and SQLite table rebuilds
```

## Usage

```rust
use toolu_orm_core::{column::Text, dialect::Dialect, error::DbCoreError};
use toolu_orm_core::query_column::{Column, CommonOps, TextOps};
use toolu_orm_core::expr::Expr;
use toolu_orm_core::{schema::SchemaRegistry, snapshot::Snapshot};

// Type-safe column references
const NAME: Column<Text> = Column::new("users", "name");
fn name_filter(name: &str) -> Expr {
    NAME.eq(name).and(NAME.like("%admin%"))
}

// Schema diffing
fn migration_sql(old: &Snapshot, new: &SchemaRegistry) -> Result<String, DbCoreError> {
    let ops = toolu_orm_core::diff::diff(old, new)?;
    Ok(toolu_orm_core::sql::generate_sql_for(&ops, Dialect::Sqlite))
}
```

`generate_sql()` selects `Dialect::CURRENT` (Postgres whenever core's
`postgres` feature is enabled); `generate_sql_for()` chooses explicitly.
Driver connection wrappers live in `toolu-orm-connection`.

## Development

```sh
cargo build -p toolu-orm-core
cargo nextest run -p toolu-orm-core
cargo clippy -p toolu-orm-core -- -D warnings
```
