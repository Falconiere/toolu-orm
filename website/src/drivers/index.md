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

## Lance extension and local namespace

The optional `lancedb` Cargo feature is forwarded by the `toolu-orm` facade to
its core, macro, query, and connection crates. It adds bundled Rust `duckdb`
`1.10505.0` (DuckDB v1.5.5). The production startup API loads and verifies
Lance extension build `2f167ea` from a caller-supplied local file:

```rust
use toolu_orm::connection::LanceConnection;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let extension = std::env::var("LANCE_EXTENSION_PATH")?;
    let lance = LanceConnection::open(std::path::Path::new(&extension))?;
    let mut statement = lance.connection().prepare(
        "SELECT loaded FROM duckdb_extensions() WHERE extension_name = 'lance'"
    )?;
    let loaded: bool = statement.query_row([], |row| row.get(0))?;
    assert!(loaded);
    Ok(())
}
```

`open` returns a loaded in-memory DuckDB connection. It never downloads or
caches an artifact, attaches a namespace, or creates a table. After startup,
attach an existing local directory to select it for unqualified SQL:

```rust
use toolu_orm::connection::{LanceColumn, LanceColumnType, LanceConnection};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let extension = std::env::var("LANCE_EXTENSION_PATH")?;
    let directory = std::env::temp_dir().join(format!(
        "toolu-lance-demo-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    std::fs::create_dir(&directory)?;

    {
        let namespace = LanceConnection::open(&extension)?.attach(&directory, "local")?;
        namespace.create_table("items", &[
            LanceColumn { name: "id", data_type: LanceColumnType::BigInt },
            LanceColumn { name: "label", data_type: LanceColumnType::Varchar },
        ])?;
        namespace.connection().execute(
            "INSERT INTO items VALUES (1, 'persisted')", []
        )?;
    }

    let reopened = LanceConnection::open(&extension)?.attach(&directory, "local")?;
    reopened.open_table("items")?;
    assert_eq!(reopened.list_tables()?, vec!["items"]);
    let label: String = reopened.connection().query_row(
        "SELECT label FROM items WHERE id = 1", [], |row| row.get(0)
    )?;
    assert_eq!(label, "persisted");
    reopened.drop_table("items")?;
    drop(reopened);
    std::fs::remove_dir_all(directory)?;
    Ok(())
}
```

`create_table` currently accepts `BigInt` and `Varchar` columns. Duplicate
creates and missing table opens or drops return named errors without replacing
existing rows. Paths are quoted or rejected before SQL runs. Catalog, table,
and column names must start with an ASCII letter or underscore and then contain
only ASCII letters, digits, or underscores; they are quoted before SQL runs.
The [namespace lifecycle scenario](https://github.com/Falconiere/toolu-orm/blob/main/docs/scenarios/lancedb-namespace-lifecycle.md)
records the real reopen and error tests. An absent,
unreadable, or incompatible file returns the named
`LanceStartupError::LanceDependencyUnavailable` before user SQL or table
mutation. The [Rust smoke probe](https://github.com/Falconiere/toolu-orm/blob/main/docs/scenarios/lancedb-rust-smoke.md)
records the pinned extension URLs and SHA-256 values for the verified macOS
arm64 and Linux amd64 artifacts. The namespace lifecycle scenario above
records the Linux arm64 checksum. Provision a matching file before an
offline run and pass its path to each new connection. The smoke script's
download lives only for that test run; it does not fill a persistent cache.
Other platforms must supply a compatible file and may receive a startup
incompatibility error. Extension paths containing backslashes are rejected.

For prepared SQL, `toolu_orm::to_duckdb_params(&values)?` converts portable
`Value` inputs before execution. It binds NULL, integer, real, text, binary,
and boolean values; `TimestampEpoch`, `TimestampText`, `Json`, `Uuid`, and
`Numeric` return `LanceValueError::Unsupported { kind }`. Pass the converted
slice to `duckdb::params_from_iter(params.iter())`. See the
[real scalar binding scenario](https://github.com/Falconiere/toolu-orm/blob/main/docs/scenarios/lancedb-scalar-binding.md)
for prepared insert and filter coverage. The tagged codecs remain outside
epic #145.

`DbConnection`, portable query execution, row decoding, and migrations are not
available for Lance yet.

`lancedb` can coexist with `postgres`, `rusqlite`, or `libsql` in Cargo. The
query crate exposes its existing executor only when one implemented driver is
enabled **without** `lancedb`; mixed feature sets have no legacy executor or
libsql transaction module. This prevents an application from accidentally
running a query through another backend while Lance is selected.

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
