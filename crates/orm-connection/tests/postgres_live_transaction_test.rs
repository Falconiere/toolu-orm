//! `PgConnection::transaction` on a live Postgres: commit persists, drop rolls
//! back, reads inside the transaction see its own writes.
//!
//! Needs `docker compose -f docker-compose.test.yaml up -d --wait` and
//! `TEST_DB_PORT=5434` locally. Compiles only on the five-crate postgres lane.
#![cfg(all(feature = "libsql", feature = "postgres"))]

#[path = "fixtures/pg_live.rs"]
mod pg_live;

use toolu_orm_connection::DbConnection;
use toolu_orm_core::value::Value;

use pg_live::{CountRow, TestResult, USERS_DDL, count_users, pg_schema_conn};

const INSERT: &str = "INSERT INTO users (id, name) VALUES ($1, $2)";

fn ann() -> Vec<Value> {
  vec![Value::Text("u1".into()), Value::Text("Ann".into())]
}

#[tokio::test]
async fn commit_persists_the_write() -> TestResult {
  let (_db, mut conn) = pg_schema_conn("conn_tx_commit").await?;
  conn.execute_batch(USERS_DDL).await?;

  let tx = conn.transaction().await?;
  assert_eq!(tx.execute_sql(INSERT, ann()).await?, 1);
  tx.commit().await?;

  assert_eq!(count_users(&conn).await?, 1);
  let named = conn
    .query_map::<CountRow>(
      "SELECT count(*) AS n FROM users WHERE name = $1",
      vec![Value::Text("Ann".into())],
    )
    .await?;
  assert_eq!(named.first().ok_or("missing row")?.n, 1);
  Ok(())
}

#[tokio::test]
async fn dropping_without_commit_rolls_back() -> TestResult {
  let (_db, mut conn) = pg_schema_conn("conn_tx_drop").await?;
  conn.execute_batch(USERS_DDL).await?;

  {
    let tx = conn.transaction().await?;
    assert_eq!(tx.execute_sql(INSERT, ann()).await?, 1);
    // `tx` dropped here without commit.
  }

  assert_eq!(count_users(&conn).await?, 0);
  Ok(())
}

#[tokio::test]
async fn reads_inside_the_transaction_see_its_own_writes() -> TestResult {
  let (_db, mut conn) = pg_schema_conn("conn_tx_own_writes").await?;
  conn.execute_batch(USERS_DDL).await?;

  let tx = conn.transaction().await?;
  tx.execute_sql(INSERT, ann()).await?;
  assert_eq!(count_users(&tx).await?, 1);
  drop(tx);

  assert_eq!(count_users(&conn).await?, 0);
  Ok(())
}

#[tokio::test]
async fn failed_statement_inside_transaction_rolls_back_on_drop() -> TestResult {
  let (_db, mut conn) = pg_schema_conn("conn_tx_failed_stmt").await?;
  conn.execute_batch(USERS_DDL).await?;

  let tx = conn.transaction().await?;
  tx.execute_sql(INSERT, ann()).await?;
  let failed = tx
    .execute_sql("INSERT INTO nope (id) VALUES ($1)", vec![Value::Integer(1)])
    .await;
  assert!(failed.is_err(), "insert into missing table must fail");
  drop(tx);

  assert_eq!(count_users(&conn).await?, 0);
  Ok(())
}
