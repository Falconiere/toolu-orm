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

For `sqlite-vec` / `vec0`, register the extension on the raw connection (or
via process-wide `sqlite3_auto_extension`) **before** `from_connection`, then
run migrate / KNN on the wrapper. The CI rusqlite lane enables the optional
`sqlite-vec` feature on `toolu-orm-query` to statically link the extension and
prove that path (`vec0_sqlite_vec_live_test`).

## Maintenance and inspection

SQLite's administration statements are not DML, so no builder reaches them:
`VACUUM`, `ATTACH`, `DETACH` and `PRAGMA` name schemas and files rather than
tables and columns. `SqliteMaintenance` is the typed replacement for building
them as strings, and it works on a **borrowed** connection — it never opens a
database, never begins a transaction, and never takes ownership, so your
connection and your transactions stay yours.

```rust
use std::path::Path;
use toolu_orm_connection::SqliteMaintenance;

// A validated pre-upgrade snapshot.
conn.vacuum_into(Path::new("/var/app/snapshot.db"))?;
let snapshot = conn.attach_database(Path::new("/var/app/snapshot.db"), "snapshot")?;
let report = snapshot.quick_check()?;
snapshot.detach()?;
if !report.is_ok() {
  return Err(format!("snapshot is unusable: {report}").into());
}

// Storage statistics.
let stats = conn.storage_stats()?;
println!("{} bytes over {} pages", stats.bytes(), stats.page_count);
```

`attach_database` returns a guard. The `DETACH` happens on every exit path,
including the `?` that abandons a half-finished copy, so the error branch you
used to have to write is gone:

```rust
let old = conn.attach_database(&archive, "old")?;
conn.execute("INSERT INTO t SELECT * FROM old.t", [])?;  // a failure here still detaches
old.detach()?;                                           // explicit, and reports its own error
```

Four things are worth knowing:

- **The filename is always a bound parameter.** SQLite treats it as an
  expression in both statements, so a path containing quotes is not a special
  case. The schema *identifier* cannot be bound — SQLite does not accept a
  parameter there — so it is rendered, always double-quoted with interior `"`
  doubled. `attach_database(path, "we\"ird")` attaches the database it names.
- **Every read names its schema.** A bare `PRAGMA quick_check` checks *all*
  attached databases, so `conn.quick_check()` issues `PRAGMA main.quick_check`
  and means your main database whatever is attached. The guard's own
  `quick_check` / `page_count` / `page_size` / `storage_stats` are its
  counterparts for one attachment.
- **Failures keep their type.** `MaintenanceError::Sqlite` holds the
  `rusqlite::Error`, result code included. Only two refusals are this API's own —
  an empty schema name and one containing a NUL byte — and neither sends a
  statement. An integrity problem is not an error at all: it comes back as a
  report whose `is_ok()` is false.
- **`Drop` cannot report.** A detach that fails while being dropped is logged at
  `warn`. Call `detach()` when you need to see it — SQLite refuses to detach a
  database your own open transaction has written to.

## Supported surface and FFI exceptions

Registering a **custom FTS5 tokenizer** — `SELECT fts5(?1)` bound through
`sqlite3_bind_pointer(.., "fts5_api_ptr", ..)`, then a C function pointer out of
`fts5_api` — is deliberately **outside** this ORM's supported surface. It is a
decision, not an omission:

1. A host pointer is not a value of any SQL type. `Value` and rusqlite's `ToSql`
   model SQL *data*, so no amount of extending them would express that handshake.
2. Supporting it means re-exporting `libsqlite3-sys` types and owning `unsafe`
   here. The workspace denies `unsafe_code`, and the single exception —
   the separately published `toolu-orm-sqlite-vec-register` — exists precisely
   so one `unsafe` registration call had somewhere to live.
3. A tokenizer belongs to a *connection*, not to a schema or a query. That is
   connection bootstrap, which this driver already delegates to you.

The sanctioned escape hatch is `with_raw_connection`, which borrows the wrapped
driver connection for one closure:

```rust
let conn = RusqliteConnection::from_connection(raw);

// Anything outside the supported surface, on the real connection.
conn.with_raw_connection(|driver| register_my_tokenizer(driver))??;

// It is also how a wrapper owner reaches the maintenance surface: the outer
// Result is the connection lock, the inner one the operation.
let stats = conn.with_raw_connection(SqliteMaintenance::storage_stats)??;
```

The lock is held for the whole closure and is not re-entrant, so the closure
must not call back into the wrapper. You can equally register before
`from_connection` — that is what the `sqlite-vec` note above does.

Everything *downstream* of registration stays fully supported: the
`#[fts5_table]` schema, `MATCH`, `bm25`, `snippet`, `highlight`, and `vec0` KNN.
Only the registration handshake is out of scope.

## Without a runtime

rusqlite is synchronous, so the connection wrapper does not have to pretend
otherwise. `DbConnectionBlocking` is the sync sibling of `DbConnection`, and
`RusqliteConnection` implements it natively — it takes the lock, calls rusqlite,
and returns. No `spawn_blocking`, no runtime:

```rust
use toolu_orm_connection::{DbConnectionBlocking, RusqliteConnection};
use toolu_orm_core::value::Value;

fn main() -> Result<(), Box<dyn std::error::Error>> {
  // `from_connection` is sync and infallible, so nothing here is async.
  let conn = RusqliteConnection::from_connection(rusqlite::Connection::open("data/app.db")?);

  conn.execute_batch("CREATE TABLE IF NOT EXISTS users (id TEXT PRIMARY KEY)")?;
  conn.execute_sql(
    "INSERT INTO users (id) VALUES (?1)",
    vec![Value::Text("u_1".to_owned())],
  )?;
  let users: Vec<User> = conn.query_map("SELECT id FROM users", vec![])?;

  println!("{} users", users.len());
  Ok(())
}
```

Two things differ from the async trait. `query_map` has no `T: Send + 'static`
bound, because the row is decoded on your own thread and never crosses one. And
the driver is usable from a plain `fn` — a CLI no longer builds a runtime just to
reach the database, and a handler that already runs its store work inside its own
`spawn_blocking` no longer pays a second thread hop.

libsql and Postgres do **not** implement `DbConnectionBlocking`: they talk to a
server and are genuinely async, so a blocking wrapper there would only hide a
`block_on`.

The async `DbConnection` impl is unchanged and still available on the same type —
it now delegates to these blocking methods inside `spawn_blocking`, so the two
surfaces run the same statement code. Because both traits are implemented for
`RusqliteConnection` and they share method names, importing both into one scope
makes `conn.execute_sql(..)` ambiguous (`E0034`). Name the trait to pick a path:

```rust
DbConnectionBlocking::execute_sql(&conn, sql, params)?;
DbConnection::execute_sql(&conn, sql, params).await?;
```

A panic that unwinds out of a statement (a `FromRow` that panics, say) poisons the
connection lock: every later call, on either trait, returns
`DbError::Connection` naming the poisoning rather than handing back a connection
that may be stuck mid-transaction.

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

This is the one driver where migrations, status, and builders share one wrapper:
`Executor for RusqliteConnection` delegates to `DbConnectionBlocking`, so the same
`from_connection` value that runs `run_migrate_blocking` / `get_status_blocking`
also runs `InsertBuilder::execute` and `SelectBuilder::fetch_all`. A bare
`rusqlite::Connection` remains a valid `Executor` for callers that never wrap.

Everything else — the builders, the expressions, the generated columns — is
identical to the other drivers.

## Dialect and transactions

rusqlite is SQLite: generate with `Dialect::Sqlite`, placeholders are `?N`.

There is no transaction wrapper for this driver. Send `BEGIN` / `COMMIT` through
`execute_batch` on the connection, or use rusqlite's own transaction API on the
raw connection.
