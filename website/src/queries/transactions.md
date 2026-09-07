# Transactions

## libsql

`TransactionExt` adds `run_transaction` to a `libsql::Connection`. The closure
receives a `Transaction` that implements the same `Executor` the builders take,
so nothing else in the call site changes:

```rust
use toolu_orm_query::transaction::TransactionExt;

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

## Postgres

Postgres transactions are explicit. `PgTransaction` wraps a
`tokio_postgres::Transaction` and implements the executor, so the same builders
run against it:

```rust
use toolu_orm_query::executor::PgTransaction;

let tx = PgTransaction::new(client.transaction().await?);

InsertBuilder::new("users")
  .set(&users::id, "u1")
  .set(&users::email, "u1@example.com")
  .execute(&tx)
  .await?;

tx.commit().await?;   // or tx.rollback().await?
```

Dropping a `PgTransaction` without calling `commit` discards the work.

The connection crate has its own wrapper for code written against
`DbConnection` rather than the query builders:

```rust
let mut conn = pg.connect().await?;
let tx = conn.transaction().await?;              // PgTransaction: DbConnection
tx.execute_sql("UPDATE counters SET n = n + 1", vec![]).await?;
tx.commit().await?;
```

## rusqlite

There is no transaction wrapper for the rusqlite driver. Issue the statements
through `execute_batch`, which sends them as one script:

```rust
// `conn` is a RusqliteConnection: DbConnection::execute_batch is async on every
// driver, rusqlite included — it runs the blocking call on a worker thread.
let conn = RusqliteConnection::open("data/app.db").await?;
conn.execute_batch("BEGIN; UPDATE counters SET n = n + 1; COMMIT;").await?;

// On a raw rusqlite::Connection it is rusqlite's own inherent method, and sync:
sqlite_conn.execute_batch("BEGIN; UPDATE counters SET n = n + 1; COMMIT;")?;
```

The wrapper has a sync path too:
`DbConnectionBlocking::execute_batch(&conn, "BEGIN; …; COMMIT;")?` sends the same
script with no runtime involved. See
[Without a runtime](../drivers/rusqlite.md#without-a-runtime).

## Migrations

`run_migrate` does not need any of this: each migration file is applied inside
its own `BEGIN` / `COMMIT`, so a failing statement rolls back that file. See
[The migration loop](../migrations/overview.md).
