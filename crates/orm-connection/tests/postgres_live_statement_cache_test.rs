//! Prepared-statement caching on a live Postgres: one `Parse` per SQL text for
//! reads, writes and transaction calls, eviction after a rejected statement or
//! a plan-invalidating DDL, and `PgDatabase::clear_statement_caches`.
//!
//! Assertions read `pg_catalog.pg_prepared_statements` on the workload's own
//! session: one row per SQL text means prepared once and kept, and
//! `generic_plans + custom_plans` counts executions.
//!
//! Needs `docker compose -f docker-compose.test.yaml up -d --wait` and
//! `TEST_DB_PORT=5434` locally. Compiles only on the five-crate postgres lane.
#![cfg(all(feature = "libsql", feature = "postgres"))]

#[path = "fixtures/pg_live.rs"]
mod pg_live;

use toolu_orm_connection::{DbConnection, DbError};
use toolu_orm_core::value::Value;
use toolu_orm_macros::FromRow;

use pg_live::{TestResult, USERS_DDL, count_users, pg_schema_conn};

const INSERT: &str = "INSERT INTO users (id, name) VALUES ($1, $2)";
const SELECT_NAME: &str = "SELECT name FROM users WHERE id = $1";
const SELECT_ALL: &str = "SELECT * FROM users WHERE id = $1";
const UPDATE_NAME: &str = "UPDATE users SET name = $1 WHERE id = $2";
const STATS: &str = "SELECT count(*) AS live, \
   coalesce(sum(generic_plans + custom_plans), 0)::bigint AS executions \
   FROM pg_prepared_statements WHERE statement = $1";

#[derive(FromRow, Debug, PartialEq)]
struct UserName {
  name: String,
}

#[derive(FromRow, Debug, PartialEq)]
struct NarrowUser {
  id: String,
  name: String,
  age: Option<i64>,
}

#[derive(FromRow, Debug, PartialEq)]
struct WideUser {
  id: String,
  name: String,
  age: Option<i64>,
  email: Option<String>,
}

#[derive(FromRow)]
struct CacheStat {
  live: i64,
  executions: i64,
}

/// `(live statements, executions)` for `sql` on this session.
async fn stat(
  conn: &impl DbConnection,
  sql: &str,
) -> Result<(i64, i64), Box<dyn std::error::Error>> {
  let rows = conn
    .query_map::<CacheStat>(STATS, vec![Value::Text(sql.into())])
    .await?;
  let row = rows.first().ok_or("cache stats returned no row")?;
  Ok((row.live, row.executions))
}

/// `users` holding `u1..un` named `name-1..name-n`, inserted one statement at a
/// time so `INSERT` is itself a repeated-SQL case.
async fn seed_users(conn: &impl DbConnection, n: usize) -> TestResult {
  conn.execute_batch(USERS_DDL).await?;
  for i in 1..=n {
    let row = vec![
      Value::Text(format!("u{i}")),
      Value::Text(format!("name-{i}")),
    ];
    assert_eq!(conn.execute_sql(INSERT, row).await?, 1);
  }
  Ok(())
}

async fn read_name(
  conn: &impl DbConnection,
  id: usize,
) -> Result<String, Box<dyn std::error::Error>> {
  let rows = conn
    .query_map::<UserName>(SELECT_NAME, vec![Value::Text(format!("u{id}"))])
    .await?;
  Ok(rows.first().ok_or("no user row")?.name.clone())
}

#[tokio::test]
async fn repeated_reads_prepare_the_statement_once() -> TestResult {
  let (_db, conn) = pg_schema_conn("conn_stmt_cache_read").await?;
  seed_users(&conn, 5).await?;

  for i in 1..=5 {
    assert_eq!(read_name(&conn, i).await?, format!("name-{i}"));
  }

  assert_eq!(stat(&conn, SELECT_NAME).await?, (1, 5), "SELECT not reused");
  assert_eq!(count_users(&conn).await?, 5);
  Ok(())
}

#[tokio::test]
async fn repeated_writes_prepare_the_statement_once() -> TestResult {
  let (_db, conn) = pg_schema_conn("conn_stmt_cache_write").await?;
  seed_users(&conn, 5).await?;

  for i in 1..=5 {
    let row = vec![
      Value::Text(format!("renamed-{i}")),
      Value::Text(format!("u{i}")),
    ];
    assert_eq!(conn.execute_sql(UPDATE_NAME, row).await?, 1);
  }

  assert_eq!(stat(&conn, UPDATE_NAME).await?, (1, 5), "UPDATE not reused");
  assert_eq!(stat(&conn, INSERT).await?, (1, 5), "INSERT not reused");
  for i in 1..=5 {
    assert_eq!(read_name(&conn, i).await?, format!("renamed-{i}"));
  }
  Ok(())
}

#[tokio::test]
async fn transaction_and_connection_share_one_cached_statement() -> TestResult {
  let (_db, mut conn) = pg_schema_conn("conn_stmt_cache_txn").await?;
  seed_users(&conn, 2).await?;

  let tx = conn.transaction().await?;
  for i in 1..=2 {
    assert_eq!(read_name(&tx, i).await?, format!("name-{i}"));
  }
  let row = vec![Value::Text("u3".into()), Value::Text("name-3".into())];
  assert_eq!(tx.execute_sql(INSERT, row).await?, 1);
  tx.commit().await?;

  assert_eq!(read_name(&conn, 3).await?, "name-3");
  // 2 reads inside + 1 outside; 2 seeding inserts + 1 inside.
  assert_eq!(stat(&conn, SELECT_NAME).await?, (1, 3), "SELECT not shared");
  assert_eq!(stat(&conn, INSERT).await?, (1, 3), "INSERT not shared");
  Ok(())
}

#[tokio::test]
async fn dropped_transaction_rolls_back_and_leaves_the_statement_cached() -> TestResult {
  let (_db, mut conn) = pg_schema_conn("conn_stmt_cache_rollback").await?;
  seed_users(&conn, 1).await?;

  {
    let tx = conn.transaction().await?;
    let row = vec![Value::Text("u9".into()), Value::Text("name-9".into())];
    assert_eq!(tx.execute_sql(INSERT, row).await?, 1);
  }

  assert_eq!(count_users(&conn).await?, 1, "the drop did not roll back");
  let row = vec![Value::Text("u2".into()), Value::Text("name-2".into())];
  assert_eq!(conn.execute_sql(INSERT, row).await?, 1);
  assert_eq!(count_users(&conn).await?, 2);
  assert_eq!(stat(&conn, INSERT).await?.0, 1, "rollback evicted it");
  Ok(())
}

#[tokio::test]
async fn a_rejected_statement_is_evicted_and_never_retried() -> TestResult {
  let (_db, conn) = pg_schema_conn("conn_stmt_cache_error").await?;
  seed_users(&conn, 1).await?;
  assert_eq!(stat(&conn, INSERT).await?, (1, 1));

  let clash = vec![Value::Text("u1".into()), Value::Text("clash".into())];
  let Err(DbError::Query(msg)) = conn.execute_sql(INSERT, clash).await else {
    return Err("a duplicate primary key did not fail".into());
  };
  assert!(msg.contains("23505"), "expected a unique violation: {msg}");
  assert_eq!(stat(&conn, INSERT).await?.0, 0, "the error kept it cached");

  let row = vec![Value::Text("u2".into()), Value::Text("name-2".into())];
  assert_eq!(conn.execute_sql(INSERT, row).await?, 1);
  assert_eq!(stat(&conn, INSERT).await?, (1, 1), "not re-prepared");
  assert_eq!(count_users(&conn).await?, 2, "the failed write was retried");
  Ok(())
}

#[tokio::test]
async fn ddl_invalidated_statement_is_evicted_then_re_prepared() -> TestResult {
  let (_db, conn) = pg_schema_conn("conn_stmt_cache_ddl").await?;
  seed_users(&conn, 1).await?;
  let id = || vec![Value::Text("u1".into())];

  let narrow = conn.query_map::<NarrowUser>(SELECT_ALL, id()).await?;
  let before = NarrowUser {
    id: "u1".into(),
    name: "name-1".into(),
    age: None,
  };
  assert_eq!(narrow, vec![before]);
  assert_eq!(stat(&conn, SELECT_ALL).await?, (1, 1));

  conn
    .execute_batch("ALTER TABLE users ADD COLUMN email TEXT")
    .await?;

  let stale = conn.query_map::<NarrowUser>(SELECT_ALL, id()).await;
  let Err(DbError::Query(msg)) = stale else {
    return Err(format!("stale cached plan did not fail: {stale:?}").into());
  };
  assert!(msg.contains("0A000"), "expected a cached-plan error: {msg}");
  assert_eq!(stat(&conn, SELECT_ALL).await?.0, 0, "no eviction");

  let wide = conn.query_map::<WideUser>(SELECT_ALL, id()).await?;
  let after = WideUser {
    id: "u1".into(),
    name: "name-1".into(),
    age: None,
    email: None,
  };
  assert_eq!(wide, vec![after], "the call after the error re-prepares");
  assert_eq!(stat(&conn, SELECT_ALL).await?, (1, 1));
  Ok(())
}

#[tokio::test]
async fn clear_statement_caches_closes_the_cached_statements() -> TestResult {
  let (db, conn) = pg_schema_conn("conn_stmt_cache_clear").await?;
  seed_users(&conn, 3).await?;
  for i in 1..=3 {
    assert_eq!(read_name(&conn, i).await?, format!("name-{i}"));
  }
  assert_eq!(stat(&conn, SELECT_NAME).await?, (1, 3));

  db.clear_statement_caches();

  assert_eq!(stat(&conn, SELECT_NAME).await?.0, 0, "statement not closed");
  assert_eq!(read_name(&conn, 2).await?, "name-2");
  assert_eq!(stat(&conn, SELECT_NAME).await?, (1, 1), "not re-prepared");
  Ok(())
}
