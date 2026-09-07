# Connections

There are two connection abstractions, and knowing which one a function wants
saves a compile error.

| Trait | Crate | Implemented for | Used by |
|---|---|---|---|
| `DbConnection` | `toolu-orm-connection` | `LibsqlConnection`, `RusqliteConnection`, `PgConnection`, `PgTransaction` | `run_migrate`, `get_status`, and your own code |
| `Executor` | `toolu-orm-query` | `libsql::Connection`, `rusqlite::Connection`, `tokio_postgres::Client`, `PgTransaction` | the query builders' `.execute()` and `fetch_*` |

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

`RusqliteConnection` from the connection crate is the async side of the same
driver: it wraps the calls in `spawn_blocking`, which is what lets it implement
the async `DbConnection` and run migrations.

## Choosing

| Driver | Reach for it when |
|---|---|
| [libsql](libsql.md) | Turso, embedded replicas, or a local SQLite file in an async application. |
| [rusqlite](rusqlite.md) | Plain local SQLite with no async runtime requirement, or tests. |
| [Postgres](postgres.md) | A server database, connection pooling, TLS. |
