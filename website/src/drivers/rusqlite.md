# rusqlite

```toml
toolu-orm-core       = { version = "0.1", default-features = false, features = ["rusqlite"] }
toolu-orm-macros     = { version = "0.1", features = ["rusqlite"] }
toolu-orm-query      = { version = "0.1", features = ["rusqlite"] }
toolu-orm-connection = { version = "0.1", features = ["rusqlite"] }
toolu-orm-cli        = { version = "0.1", default-features = false, features = ["rusqlite"] }
```

rusqlite is bundled, so there is no system SQLite to install.

## Opening

```rust
use toolu_orm_connection::RusqliteConnection;

let conn = RusqliteConnection::open("data/app.db").await?;
let mem  = RusqliteConnection::open_in_memory().await?;
```

Both constructors are `async` even though the driver is not: the work runs on a
blocking thread, which is what lets `RusqliteConnection` implement the async
`DbConnection` and be handed to `run_migrate`.

## The builders are synchronous here

The `Executor` impl is on `rusqlite::Connection` itself, and it is sync. With the
`rusqlite` feature there is nothing to await:

```rust
let n = InsertBuilder::new("users")
  .set(&users::id, "u_1")
  .execute(&sqlite_conn)?;                // no .await

let rows: Vec<User> = UsersTable::select_for::<User>()
  .filter(users::id.eq("u_1"))
  .fetch_all(&sqlite_conn)?;              // no .await
```

Everything else — the builders, the expressions, the generated columns — is
identical to the other drivers.

## Dialect and transactions

rusqlite is SQLite: generate with `Dialect::Sqlite`, placeholders are `?N`.

There is no transaction wrapper for this driver. Send `BEGIN` / `COMMIT` through
`execute_batch` on the connection, or use rusqlite's own transaction API on the
raw connection.
