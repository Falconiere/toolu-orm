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

## Adopting a connection you configured

`open` and `open_in_memory` give you a connection with rusqlite's defaults.
When you need something else — a pragma, an open flag, a loaded extension, an
attached database — configure a `rusqlite::Connection` yourself and hand it
over:

```rust
let raw = rusqlite::Connection::open_with_flags(
  "data/app.db",
  rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
)?;
raw.execute_batch("PRAGMA busy_timeout = 5000;")?;

let conn = RusqliteConnection::from_connection(raw);
```

`from_connection` is neither `async` nor fallible: the connection is already
open, so wrapping it cannot block or fail. It moves that exact connection into
the wrapper, so everything set on it beforehand stays in effect and nothing is
reset. A connection that turns out to be unusable — a read-only one being
written to, say — reports it at the first statement as `DbError::Query`.

## The builders are synchronous here

The `Executor` impl is on `rusqlite::Connection` itself, and it is sync. With the
`rusqlite` feature there is nothing to await:

```rust
// Your own raw connection. `from_connection` only goes inwards and it takes
// ownership, so a connection handed to the wrapper is no longer yours to use
// here — open this one separately.
let sqlite_conn = rusqlite::Connection::open("data/app.db")?;

let n = InsertBuilder::new("users")
  .set(&users::id, "u_1")
  .execute(&sqlite_conn)?;                // no .await

let rows: Vec<User> = UsersTable::select_for::<User>()
  .filter(users::id.eq("u_1"))
  .fetch_all(&sqlite_conn)?;              // no .await
```

This is the one driver where the two surfaces do not connect:
`RusqliteConnection` holds its `rusqlite::Connection` behind an
`Arc<Mutex<…>>` with no accessor, so it serves `DbConnection` (migrations,
`query_map`, `execute_batch`) while the builders run on a `rusqlite::Connection`
you open directly. `from_connection` does not bridge them — it is the way in,
not a way back out. Point both at the same database file, or use one of them.

Everything else — the builders, the expressions, the generated columns — is
identical to the other drivers.

## Dialect and transactions

rusqlite is SQLite: generate with `Dialect::Sqlite`, placeholders are `?N`.

There is no transaction wrapper for this driver. Send `BEGIN` / `COMMIT` through
`execute_batch` on the connection, or use rusqlite's own transaction API on the
raw connection.
