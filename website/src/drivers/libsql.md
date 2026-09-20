# libsql and Turso

```toml
toolu-orm-core       = { version = "0.9", default-features = false, features = ["libsql"] }
toolu-orm-macros     = { version = "0.9", features = ["libsql"] }
toolu-orm-query      = { version = "0.9", features = ["libsql"] }
toolu-orm-connection = { version = "0.9", features = ["libsql"] }
toolu-orm-cli        = { version = "0.9", default-features = false, features = ["libsql"] }
```

## Local and in-memory

```rust
use toolu_orm_connection::Database;

let db = Database::init_local("data/app.db").await?;   // or ":memory:"
let conn = db.connect()?;                              // LibsqlConnection
let exec = conn.inner_conn();                          // &libsql::Connection
```

`:memory:` is the usual choice for tests — a fresh database per test, no cleanup,
still a real SQL engine.

## Turso embedded replica

`init_remote` configures libsql's synced database builder with a local replica
file, a remote URL, and a background sync interval. It completes an initial sync
before returning the database.

```rust
use toolu_orm_connection::{Database, RemoteConfig};

let db = Database::init_remote(RemoteConfig {
  replica_path: "data/app.db".into(),
  url: "libsql://my-db.turso.io".into(),
  auth_token: std::env::var("TURSO_TOKEN")?,
  sync_interval_secs: 5,
  max_sync_attempts: 5,
}).await?;
```

`sync_interval_secs` is the background sync period; `max_sync_attempts` bounds
the number of initial sync attempts. Ordinary failures retry with exponential
backoff starting at 500 ms. Replica conflicts or generation-ID mismatches cause
the local replica files to be removed and rebuilt. Exhaustion returns
`DbError::Connection`; this setting does not impose a timeout on each attempt.

## Dialect

libsql is SQLite: generate migrations with `Dialect::Sqlite`, and the builders
render `?N` placeholders. `#[table(name = "users", strict = true)]` emits a
`STRICT` table. Check the [column type mapping](../schema/column-types.md)
before enabling it: some marker types emit extension type names that stock
SQLite STRICT tables reject.

## Transactions

`TransactionExt::run_transaction` is libsql-only and commits on `Ok`, rolls back
on `Err`. See [Transactions](../queries/transactions.md).
