# toolu-orm-query

Type-safe SQL query builders for toolu-orm. Provides Select, Insert, Update, and Delete builders with feature-gated database executors for libsql (async) or rusqlite (sync).

## Stack

- **Query Building:** toolu-orm-core (Expr, Column\<T\>, Value)
- **Execution:** libsql (async, default) or rusqlite (sync)
- **Async:** async-trait, tokio (feature: libsql)

## Architecture

```
src/
├── lib.rs                       # Module exports
├── select/
│   ├── builder.rs               # SelectBuilder (columns, joins, filters, limit)
│   ├── projection.rs            # column_expr / column_scalar and the select list
│   ├── ordering.rs              # order_by and the ORDER BY tail
│   └── executor_fetch.rs        # query_map extension methods (feature-gated)
├── insert.rs                    # InsertBuilder (set, set_scalar, or_replace, or_ignore)
├── update.rs                    # UpdateBuilder (set, set_expr, set_scalar, filter)
├── delete.rs                    # DeleteBuilder (filter)
├── executor.rs                  # Executor trait + database implementations
├── transaction.rs               # Transaction wrapper
├── where_clause.rs              # Filter macro + WHERE generation
├── error.rs                     # QueryError
└── exec_helpers.rs              # impl_execute! macro
```

## Usage

```rust
use toolu_orm_query::select::SelectBuilder;
use toolu_orm_query::insert::InsertBuilder;
use toolu_orm_core::query_column::CommonOps;

// Select with type-safe columns
let (sql, params) = SelectBuilder::new("users")
    .columns_typed(&[&users::columns::ID, &users::columns::EMAIL])
    .filter(users::columns::EMAIL.eq("alice@example.com"))
    .order_by(users::columns::CREATED_AT.desc())
    .limit(10)
    .to_sql();

// Aliased tables: users sharing an email, oldest first (a self-join)
let older = TableRef::aliased("users", "older");
let newer = TableRef::aliased("users", "newer");
let (sql, params) = SelectBuilder::from_table(&older)
    .column_as(&older.column(&users::columns::ID), "older_id")
    .column_as(&newer.column(&users::columns::ID), "newer_id")
    .join(
        &newer,
        older
            .column(&users::columns::EMAIL)
            .equals(&newer.column(&users::columns::EMAIL))
            .and(
                older
                    .column(&users::columns::CREATED_AT)
                    .less_than(&newer.column(&users::columns::CREATED_AT)),
            ),
    )
    .to_sql();

// Insert
let rows = InsertBuilder::new("users")
    .set(&users::columns::ID, "uuid-123")
    .set(&users::columns::EMAIL, "alice@example.com")
    .execute(&conn)
    .await?;
```

## Development

```sh
cargo build -p toolu-orm-query
cargo nextest run -p toolu-orm-query
cargo clippy -p toolu-orm-query -- -D warnings
```
