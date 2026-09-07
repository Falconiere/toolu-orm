# libsql and Turso

```toml
toolu-orm-core       = { version = "0.1", default-features = false, features = ["libsql"] }
toolu-orm-macros     = { version = "0.1", features = ["libsql"] }
toolu-orm-query      = { version = "0.1", features = ["libsql"] }
toolu-orm-connection = { version = "0.1", features = ["libsql"] }
toolu-orm-cli        = { version = "0.1", default-features = false, features = ["libsql"] }
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

`init_remote` opens a local replica file kept in sync with a remote database.
Reads hit the local file; writes go to the remote and come back on the next sync.

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
the retries on the initial sync, so a replica that starts while the network is
down fails with `DbError::Connection` instead of hanging.

## Dialect

libsql is SQLite: generate migrations with `Dialect::Sqlite`, and the builders
render `?N` placeholders. `#[table(strict = true)]` emits a Turso `STRICT` table,
which is worth turning on — it rejects values that do not match the column type
instead of silently storing them.

## Transactions

`TransactionExt::run_transaction` is libsql-only and commits on `Ok`, rolls back
on `Err`. See [Transactions](../queries/transactions.md).
