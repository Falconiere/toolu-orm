# Transactions

Query executors and their transaction APIs require exactly one driver feature
on `toolu-orm-query`.

## libsql

`TransactionExt` adds `run_transaction` to a `libsql::Connection`. The closure
receives a `Transaction` that implements the same `Executor` the builders take,
so nothing else in the call site changes:

```rust
use toolu_orm_query::transaction::TransactionExt;
use toolu_orm_query::{insert::InsertBuilder, update::UpdateBuilder};

conn.run_transaction(|tx| async move {
  InsertBuilder::new("users")
    .set(&users::id, "tx1")
    .set(&users::email, "tx@example.com")
    .execute(&tx)
    .await?;

  UpdateBuilder::new("counters")
    .set_expr(&counters::users, "users + 1")
    .execute(&tx)
    .await?;

  Ok(())
}).await?;
```

Return `Ok` and the transaction commits. Return `Err` — including the `?` on any
statement inside — and it is dropped without committing, which rolls it back.
Reads inside the closure see the closure's own writes.
When using `LibsqlConnection` from the connection crate, call
`conn.inner_conn().run_transaction(...)`.

## Postgres

Postgres transactions are explicit. `PgTransaction` wraps a
`tokio_postgres::Transaction` and implements the executor, so the same builders
run against it:

```rust
use toolu_orm_query::executor::PgTransaction;

// client is a mutable tokio_postgres::Client.
let tx = PgTransaction::new(client.transaction().await?);

InsertBuilder::new("users")
  .set(&users::id, "u1")
  .set(&users::email, "u1@example.com")
  .execute(&tx)
  .await?;

tx.commit().await?;   // or tx.rollback().await?
```

Dropping a `PgTransaction` without calling `commit` discards the work.

The connection crate has a separate wrapper for code written against
`DbConnection`. This is a different `PgTransaction` type and does not implement
the query crate's `Executor`:

```rust
use toolu_orm_connection::DbConnection;

let mut conn = pg.connect().await?;
let tx = conn.transaction().await?;              // PgTransaction: DbConnection
tx.execute_sql("UPDATE counters SET n = n + 1", vec![]).await?;
tx.commit().await?;
```

## rusqlite

Use the driver's transaction on a mutable raw `rusqlite::Connection`. Pass
`&*tx` to a builder: `*tx` dereferences the transaction to its connection, and
`&` borrows that connection:

```rust
let tx = sqlite_conn.transaction()?;
InsertBuilder::new("users")
  .set(&users::id, "tx1")
  .set(&users::email, "tx@example.com")
  .execute(&*tx)?;
UpdateBuilder::new("counters")
  .set_expr(&counters::users, "users + 1")
  .execute(&*tx)?;
tx.commit()?; // or tx.rollback()?
```

With the driver's default drop behavior, leaving scope without committing rolls
the transaction back. These builder calls are synchronous.

`RusqliteConnection` from the connection crate has no transaction wrapper.
Its `DbConnection::execute_batch` is async and its
`DbConnectionBlocking::execute_batch` is synchronous. A script containing
`BEGIN; …; COMMIT;` needs explicit error handling: a failed statement can leave
the transaction open before `COMMIT` is reached, so handle `ROLLBACK` on failure.
See [Without a runtime](../drivers/rusqlite.md#without-a-runtime).

## Migrations

`run_migrate` does not need any of this: each migration file is applied inside
its own `BEGIN` / `COMMIT`, so a failing statement rolls back that file. See
[The migration loop](../migrations/overview.md).
