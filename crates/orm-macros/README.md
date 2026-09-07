# toolu-orm-macros

Procedural macros for toolu-orm. Transforms annotated structs into full ORM entities with schema definitions, type-safe column constants, view types, and row mapping.

## Stack

- **Proc-macro:** syn 2, quote 1, proc-macro2 1
- **Schema:** toolu-orm-core (TableDef, ColumnDef, ColumnType)

## Architecture

```
src/
├── lib.rs                       # 3 proc macros: table, FromRow, ColumnEnum
├── column_enum.rs               # #[derive(ColumnEnum)] implementation
├── from_row.rs                  # #[derive(FromRow)] implementation
├── view.rs                      # #[view] attribute processing
├── parse/
│   ├── column_parsing.rs        # Parse #[column(...)] attributes
│   └── index_parsing.rs         # Parse #[index] / #[unique_index]
└── expand/
    ├── schema_expansion.rs      # Generate TableSchema impl
    └── columns_expansion.rs     # Generate Column<T> constants module
```

## Usage

```rust
use toolu_orm_macros::table;
use toolu_orm_core::column::*;

#[table(name = "users")]
#[index(name = "idx_email", columns = ["email"], unique = true)]
pub struct UserRow {
    #[column(primary_key, not_null)]
    pub id: Text,

    #[column(not_null)]
    pub email: Text,

    #[column(default = "CURRENT_TIMESTAMP")]
    pub created_at: Timestamp,
}

// Generated: UserRow::table_def(), UserRow::columns::ID, UserRow::columns::EMAIL, etc.
```

## Development

```sh
cargo build -p toolu-orm-macros
cargo nextest run -p toolu-orm-macros
cargo clippy -p toolu-orm-macros -- -D warnings
```
