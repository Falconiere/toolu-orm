# Connections

There are three connection abstractions, and knowing which one a function wants
saves a compile error.

| Trait | Crate | Implemented for | Used by |
|---|---|---|---|
| `DbConnection` | `toolu-orm-connection` | `LibsqlConnection`, `RusqliteConnection`, `PgConnection`, `toolu_orm_connection::PgTransaction` | `run_migrate`, `get_status`, and your own code |
| `DbConnectionBlocking` | `toolu-orm-connection` | `RusqliteConnection` only | blocking migration/status APIs and your own code |
| `Executor` | `toolu-orm-query` | `libsql::Connection`, `rusqlite::Connection`, `RusqliteConnection`, `tokio_postgres::Client`, `toolu_orm_query::transaction::Transaction` (libsql only), `toolu_orm_query::executor::PgTransaction` (Postgres only) | the query builders' `.execute()` and `fetch_*` |

The first and last rows include a `PgTransaction`, and they are **two different types** — the
connection crate's wrapper implements `DbConnection`, the query crate's
implements `Executor`. The connection wrapper holds a deadpool transaction;
the query wrapper holds a `tokio_postgres::Transaction`. Builders take the query
crate's wrapper; functions accepting `DbConnection` take the connection crate's.

`Executor` and the builders' execution/fetch methods are available only when
**exactly one** driver feature is enabled on `toolu-orm-query`. The connection
crate can enable several drivers together. Its blocking trait also supports
`run_migrate_blocking` and `get_status_blocking` without an async runtime.

`DbConnection` is the portable, driver-agnostic surface:

```rust
#[async_trait::async_trait]
pub trait DbConnection: Send + Sync {
  async fn execute_sql(&self, sql: &str, params: Vec<Value>) -> Result<u64, DbError>;
  async fn query_map<T: FromRow + Send + 'static>(&self, sql: &str, params: Vec<Value>)
    -> Result<Vec<T>, DbError>;
  async fn execute_batch(&self, sql: &str) -> Result<(), DbError>;
}
```

Take `&impl DbConnection` in your own repository functions and the driver becomes
a caller choice. Raw SQL still needs the selected database's syntax and
placeholders (`?1` for SQLite, `$1` for Postgres).

`DbConnectionBlocking` is the same surface without the `async`, and only rusqlite
implements it — it provides a synchronous SQLite API that can run a statement
without a runtime:

```rust
pub trait DbConnectionBlocking: Send + Sync {
  fn execute_sql(&self, sql: &str, params: Vec<Value>) -> Result<u64, DbError>;
  fn query_map<T: FromRow>(&self, sql: &str, params: Vec<Value>) -> Result<Vec<T>, DbError>;
  fn execute_batch(&self, sql: &str) -> Result<(), DbError>;
}
```

Note the missing `Send + 'static` on `T`: rows are decoded on the calling thread.
See [rusqlite](rusqlite.md#without-a-runtime).

`Executor` is the narrower trait the builders run on. For libsql, borrow the
raw connection from the wrapper:

```rust
let db = Database::init_local("data/app.db").await?;
let conn = db.connect()?;          // LibsqlConnection — DbConnection, for migrations
let exec = conn.inner_conn();      // &libsql::Connection — Executor, for builders

run_migrate(&conn, "migrations", Dialect::Sqlite).await?;
UsersTable::insert().set(&users::id, "u_1").execute(exec).await?;
```

`RusqliteConnection` implements `Executor` directly. `PgConnection` does not
implement it or expose its pooled client; Postgres builders need a separate
`tokio_postgres::Client` or the query crate's transaction wrapper.

## Sync and async

libsql and Postgres are async, so `.execute()` and `fetch_*` are `async` and take
`.await`. rusqlite is synchronous: with the `rusqlite` feature the same methods
are **not** async and are called without `.await`.

```rust
// libsql / postgres
let users: Vec<User> = UsersTable::select_for::<User>().fetch_all(exec).await?;

// rusqlite
let users: Vec<User> = UsersTable::select_for::<User>().fetch_all(&sqlite_conn)?;
```

`RusqliteConnection` from the connection crate implements both: `DbConnection`,
by wrapping the calls in `spawn_blocking` so it can run migrations alongside the
async drivers, and `DbConnectionBlocking` natively, for consumers with no runtime.
The async impl delegates to the blocking one, so the two cannot drift.

## Choosing

| Driver | Reach for it when |
|---|---|
| [libsql](libsql.md) | Turso, embedded replicas, or a local SQLite file in an async application. |
| [rusqlite](rusqlite.md) | Plain local SQLite with no async runtime requirement (`DbConnectionBlocking`), or tests. |
| [Postgres](postgres.md) | A server database, connection pooling, TLS. |
