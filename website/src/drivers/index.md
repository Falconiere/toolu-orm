# Connections

There are three connection abstractions, and knowing which one a function wants
saves a compile error.

| Trait | Crate | Implemented for | Used by |
|---|---|---|---|
| `DbConnection` | `toolu-orm-connection` | `LibsqlConnection`, `RusqliteConnection`, `PgConnection`, `toolu_orm_connection::PgTransaction` | `run_migrate`, `get_status`, and your own code |
| `DbConnectionBlocking` | `toolu-orm-connection` | `RusqliteConnection` only | your own code, with no runtime |
| `Executor` | `toolu-orm-query` | `libsql::Connection`, `rusqlite::Connection`, `tokio_postgres::Client`, `toolu_orm_query::executor::PgTransaction` | the query builders' `.execute()` and `fetch_*` |

Both rows end in a `PgTransaction`, and they are **two different types** — the
connection crate's wrapper implements `DbConnection`, the query crate's
implements `Executor`. Neither is `tokio_postgres::Transaction`; both wrap one.
Pick by what you are calling: builders take the query crate's, `run_migrate` and
`execute_sql` take the connection crate's.

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
a Cargo feature rather than a rewrite.

`DbConnectionBlocking` is the same surface without the `async`, and only rusqlite
implements it — it is the one in-process driver, so it is the one that can run a
statement without a runtime:

```rust
pub trait DbConnectionBlocking: Send + Sync {
  fn execute_sql(&self, sql: &str, params: Vec<Value>) -> Result<u64, DbError>;
  fn query_map<T: FromRow>(&self, sql: &str, params: Vec<Value>) -> Result<Vec<T>, DbError>;
  fn execute_batch(&self, sql: &str) -> Result<(), DbError>;
}
```

Note the missing `Send + 'static` on `T`: rows are decoded on the calling thread.
See [rusqlite](rusqlite.md#without-a-runtime).

`Executor` is the narrower trait the builders run on, and it is implemented on
the **driver's own connection type**. The wrappers expose it:

```rust
let db = Database::init_local("data/app.db").await?;
let conn = db.connect()?;          // LibsqlConnection — DbConnection, for migrations
let exec = conn.inner_conn();      // &libsql::Connection — Executor, for builders

run_migrate(&conn, "migrations", Dialect::Sqlite).await?;
UsersTable::insert().set(&users::id, "u_1").execute(exec).await?;
```

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
