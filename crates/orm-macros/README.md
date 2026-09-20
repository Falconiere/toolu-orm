# toolu-orm-macros

Procedural macros for toolu-orm. Transforms annotated structs into full ORM entities with schema definitions, type-safe column constants, view types, and row mapping.

| Macro | Generates |
|---|---|
| `#[table]` | Ordinary table metadata, typed columns and query factories |
| `#[fts5_table]` | SQLite FTS5 metadata, typed columns and query factories |
| `#[vec0_table]` | SQLite vec0 metadata, typed columns and query factories |
| `#[derive(FromRow)]` | Positional row decoding for the drivers enabled on core |
| `#[derive(ColumnEnum)]` | `EnumSchema` variant strings for TEXT/CHECK columns |
| `#[derive(Relational)]` | Scalar-column metadata and JSON decoding for nested results |

Relation joins are configured separately through `RelationalQuery`; the derive
does not generate join builders from relation attributes.

## Stack

- **Proc-macro:** syn 2, quote 1, proc-macro2 1
- **Schema:** toolu-orm-core (TableDef, ColumnDef, ColumnType)

## Architecture

```
src/
├── lib.rs                       # Six proc-macro entry points
├── column_enum.rs               # #[derive(ColumnEnum)] implementation
├── from_row.rs                  # #[derive(FromRow)] field/attribute parsing
├── from_row_expand.rs           # Per-driver row reads + impl_derived_from_row! call
├── paths.rs                     # Absolute, consumer-resolved crate paths
├── view.rs                      # #[view] attribute processing
├── fts5.rs                      # #[fts5_table] parsing and expansion
├── vec0/                        # #[vec0_table] parsing and expansion
├── relational.rs               # #[derive(Relational)] parsing
├── parse/
│   ├── column_parsing.rs        # Struct fields -> column definitions
│   ├── column_attrs.rs          # Read and strip #[column(...)] attributes
│   ├── column_type_spec.rs      # Field type -> TypeSpec (Varchar<N>, Char<N>)
│   ├── primary_key_parsing.rs   # Table-level composite primary keys
│   └── index_parsing.rs         # Parse #[index] / #[unique_index]
└── expand/
    ├── schema_expansion.rs      # Generate TableSchema impl
    ├── columns_expansion.rs     # Generate Column<T> constants module
    └── relational_expansion.rs  # Generate scalar metadata and row decoding
```

## Usage

With direct dependencies, table attributes need `toolu-orm-core`,
`toolu-orm-macros` and `toolu-orm-query`, because they generate builder methods.
The `toolu-orm` facade supplies all three. Macro paths resolve automatically
through either dependency style, including Cargo renames.

```rust
use toolu_orm_macros::table;
use toolu_orm_core::column::{Integer, Text};
use toolu_orm_core::table::TableSchema;

#[table(name = "users")]
#[unique_index("idx_email", email)]
#[view(UserPublic, pick(id, email))]
pub struct UserRow {
    #[column(primary_key, not_null)]
    pub id: Text,

    #[column(not_null)]
    pub email: Text,

    #[column(not_null, default = "unixepoch()")]
    pub created_at: Integer,
}

fn main() {
    let schema = UserRow::table_def();
    assert_eq!(schema.name, users::TABLE);
    assert_eq!(users::ALL_COLUMNS, &["id", "email", "created_at"]);
    // users::id and users::email are Column<Text> constants.
    let _query = UserRow::select();
}
```

Additional ordinary-table attributes include composite
`#[primary_key(a, b)]`, `#[column(primary_key, autoincrement)]` on `Integer`,
`#[column(check = "score >= 0")]`, and partial indexes such as
`#[index("active_email", email, where = "deleted_at IS NULL")]`.

Views map markers to data fields and derive Serde, `Debug` and `Clone`; they do
not implement `FromRow`. Declare a separate read struct with that derive or
implement the trait by hand for the generated view.

## Development

```sh
cargo build -p toolu-orm-macros
cargo nextest run -p toolu-orm-macros
cargo clippy -p toolu-orm-macros -- -D warnings
```
