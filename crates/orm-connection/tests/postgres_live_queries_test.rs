//! `execute_sql`, `execute_batch`, and `query_map` on a live Postgres: error
//! mapping and parameter binding for every `Value` variant.
//!
//! Needs `docker compose -f docker-compose.test.yaml up -d --wait` and
//! `TEST_DB_PORT=5434` locally. Compiles only on the five-crate postgres lane.
#![cfg(all(feature = "libsql", feature = "postgres"))]

#[path = "fixtures/pg_live.rs"]
mod pg_live;

use toolu_orm_connection::{DbConnection, DbError};
use toolu_orm_core::value::Value;

use pg_live::{CountRow, TestResult, USERS_DDL, count_users, pg_schema_conn};

#[tokio::test]
async fn execute_sql_on_missing_table_is_query_error_with_sqlstate() -> TestResult {
  let (_db, conn) = pg_schema_conn("conn_q_bad_sql").await?;
  let result = conn
    .execute_sql(
      "INSERT INTO nope (id) VALUES ($1)",
      vec![Value::Text("x".into())],
    )
    .await;
  let Err(DbError::Query(msg)) = result else {
    return Err("expected DbError::Query".into());
  };
  assert!(msg.starts_with("ERROR:"), "severity missing: {msg}");
  assert!(msg.contains("42P01"), "sqlstate missing: {msg}");
  Ok(())
}

#[tokio::test]
async fn execute_batch_runs_every_statement() -> TestResult {
  let (_db, conn) = pg_schema_conn("conn_q_batch").await?;
  conn
    .execute_batch(&format!(
      "{USERS_DDL}; CREATE TABLE posts (id TEXT PRIMARY KEY); \
       INSERT INTO users (id, name) VALUES ('u1', 'Ann'); \
       INSERT INTO posts (id) VALUES ('p1')"
    ))
    .await?;
  assert_eq!(count_users(&conn).await?, 1);
  let posts = conn
    .query_map::<CountRow>("SELECT count(*) AS n FROM posts", vec![])
    .await?;
  assert_eq!(posts.first().ok_or("missing row")?.n, 1);
  Ok(())
}

#[tokio::test]
async fn execute_batch_failure_leaves_nothing_behind() -> TestResult {
  let (_db, conn) = pg_schema_conn("conn_q_batch_fail").await?;
  let result = conn
    .execute_batch(&format!("{USERS_DDL}; INSERT INTO nope VALUES (1)"))
    .await;
  assert!(matches!(result, Err(DbError::Query(_))), "got {result:?}");
  // A simple-query batch is one implicit transaction, so the table is gone too.
  let Err(DbError::Query(msg)) = conn
    .query_map::<CountRow>("SELECT count(*) AS n FROM users", vec![])
    .await
  else {
    return Err("users table survived a failed batch".into());
  };
  assert!(msg.contains("42P01"), "expected undefined_table: {msg}");
  Ok(())
}

#[tokio::test]
async fn query_map_type_mismatch_is_row_mapping_error() -> TestResult {
  let (_db, conn) = pg_schema_conn("conn_q_type_mismatch").await?;
  let result = conn
    .query_map::<CountRow>("SELECT 'not a number' AS n", vec![])
    .await;
  let Err(DbError::RowMapping(msg)) = result else {
    return Err("expected DbError::RowMapping".into());
  };
  assert!(msg.contains("column 0 (n)"), "unexpected message: {msg}");
  Ok(())
}

#[tokio::test]
async fn query_map_binds_every_value_variant() -> TestResult {
  let (_db, conn) = pg_schema_conn("conn_q_values").await?;
  conn
    .execute_batch(
      "CREATE TABLE v (id BIGINT PRIMARY KEY, t TEXT, r DOUBLE PRECISION, b BYTEA, n BIGINT)",
    )
    .await?;
  conn
    .execute_sql(
      "INSERT INTO v (id, t, r, b, n) VALUES ($1, $2, $3, $4, $5)",
      vec![
        Value::Integer(i64::MAX),
        Value::Text("héllo".into()),
        Value::Real(1.5),
        Value::Blob(vec![0, 255, 7]),
        Value::Null,
      ],
    )
    .await?;
  let rows = conn
    .query_map::<CountRow>(
      "SELECT count(*) AS n FROM v WHERE id = $1 AND t = $2 AND r = $3 AND b = $4 AND n IS NULL",
      vec![
        Value::Integer(i64::MAX),
        Value::Text("héllo".into()),
        Value::Real(1.5),
        Value::Blob(vec![0, 255, 7]),
      ],
    )
    .await?;
  assert_eq!(rows.first().ok_or("missing row")?.n, 1);
  Ok(())
}
