//! `toolu_orm_query::executor::PgTransaction` on a live Postgres: commit
//! persists, explicit rollback and drop discard, reads inside see own writes.
//!
//! Needs `docker compose -f docker-compose.test.yaml up -d --wait` and
//! `TEST_DB_PORT=5434` locally; compiles only via the five-crate postgres lane.

#[path = "fixtures/postgres_db.rs"]
pub mod pg;

use toolu_orm_query::executor::PgTransaction;

use pg::{all_users, client, insert_user, TestResult};

#[tokio::test]
async fn commit_persists_the_write() -> TestResult {
  let mut client = client("q_pg_tx_commit").await?;
  let tx = PgTransaction::new(client.transaction().await?);
  assert_eq!(insert_user(&tx, "u1", "Ann", "a@x.io", None).await?, 1);
  tx.commit().await?;
  assert_eq!(all_users(&client).await?.len(), 1);
  Ok(())
}

#[tokio::test]
async fn rollback_discards_the_write() -> TestResult {
  let mut client = client("q_pg_tx_rollback").await?;
  let tx = PgTransaction::new(client.transaction().await?);
  insert_user(&tx, "u1", "Ann", "a@x.io", None).await?;
  tx.rollback().await?;
  assert!(all_users(&client).await?.is_empty());
  Ok(())
}

#[tokio::test]
async fn dropping_without_commit_discards_the_write() -> TestResult {
  let mut client = client("q_pg_tx_drop").await?;
  {
    let tx = PgTransaction::new(client.transaction().await?);
    insert_user(&tx, "u1", "Ann", "a@x.io", None).await?;
  }
  assert!(all_users(&client).await?.is_empty());
  Ok(())
}

#[tokio::test]
async fn reads_inside_the_transaction_see_its_own_writes() -> TestResult {
  let mut client = client("q_pg_tx_reads").await?;
  let tx = PgTransaction::new(client.transaction().await?);
  insert_user(&tx, "u1", "Ann", "a@x.io", None).await?;
  assert_eq!(all_users(&tx).await?.len(), 1);
  tx.rollback().await?;
  assert!(all_users(&client).await?.is_empty());
  Ok(())
}
