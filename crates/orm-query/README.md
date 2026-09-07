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
│   ├── builder.rs               # SelectBuilder (columns, joins, filters, order, limit)
│   └── executor_fetch.rs        # query_map extension methods (feature-gated)
├── insert.rs                    # InsertBuilder (set, or_replace, or_ignore)
├── update.rs                    # UpdateBuilder (set, set_expr, filter)
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
