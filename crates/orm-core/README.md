# toolu-orm-core

Core types and schema engine for toolu-orm. Provides column types, table definitions, value conversions, type-safe expression building, schema snapshotting, diffing, and SQL generation.

## Stack

- **Serialization:** serde + serde_json (schema snapshots, journals)
- **Hashing:** sha2 (migration integrity)
- **Database:** libsql or rusqlite (feature-gated, optional)

## Architecture

```
src/
├── lib.rs              # Module exports
├── column.rs           # ColumnType, ColumnDef, marker types
├── table.rs            # TableDef, TableSchema trait
├── schema.rs           # SchemaRegistry
├── index.rs            # IndexDef
├── value.rs            # Value enum + driver conversions
├── row.rs              # FromRow trait (feature-gated)
├── expr.rs             # Expr, ExprKind, OrderBy, JoinCondition
├── query_column.rs     # Column<T>, ColumnRef, CommonOps/TextOps/NumericOps
├── connection.rs       # libsql Connection wrapper (feature: libsql)
├── error.rs            # DbCoreError
├── diff.rs             # Schema diff → Operation list
├── journal.rs          # Migration journal (entries + SHA256 hashes)
├── snapshot.rs         # Schema snapshot serialization
└── sql.rs              # SQL generation from Operations
```

## Usage

```rust
use toolu_orm_core::{column::ColumnType, table::TableDef, schema::SchemaRegistry};
use toolu_orm_core::query_column::{Column, CommonOps, TextOps};
use toolu_orm_core::expr::Expr;
use toolu_orm_core::value::Value;

// Type-safe column references
const NAME: Column<toolu_orm_core::column::Text> = Column::new("users", "name");
let filter = NAME.eq("Alice").and(NAME.like("%admin%"));

// Schema diffing
let ops = toolu_orm_core::diff::diff(&old_snapshot, &new_registry);
let sql = toolu_orm_core::sql::generate_sql(&ops);
```

## Development

```sh
cargo build -p toolu-orm-core
cargo nextest run -p toolu-orm-core
cargo clippy -p toolu-orm-core -- -D warnings
```
