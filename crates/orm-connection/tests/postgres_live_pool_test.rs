//! `PgDatabase::init` / `connect` against a live Postgres: round trip, pool
//! sessions, unreachable server.
//!
//! Needs `docker compose -f docker-compose.test.yaml up -d --wait` and
//! `TEST_DB_PORT=5434` locally (CI provides a service container). Compiles only
//! on the five-crate postgres lane, where orm-core has the postgres+libsql
//! `FromRow` shape.
#![cfg(all(feature = "libsql", feature = "postgres"))]

#[path = "fixtures/pg_live.rs"]
mod pg_live;

use toolu_orm_connection::{DbConnection, DbError, PgConfig, PgDatabase};
use toolu_orm_core::value::Value;

use pg_live::{CountRow, TestResult, USERS_DDL, count_users, pg_schema_conn};

#[tokio::test]
async fn init_connect_insert_and_read_back() -> TestResult {
  let (_db, conn) = pg_schema_conn("conn_pool_round_trip").await?;
  conn.execute_batch(USERS_DDL).await?;

  let affected = conn
    .execute_sql(
      "INSERT INTO users (id, name, age) VALUES ($1, $2, $3)",
      vec![
        Value::Text("u1".into()),
        Value::Text("Ann".into()),
        Value::Integer(41),
      ],
    )
    .await?;
  assert_eq!(affected, 1);

  let rows = conn
    .query_map::<CountRow>(
      "SELECT count(*) AS n FROM users WHERE name = $1",
      vec![Value::Text("Ann".into())],
    )
    .await?;
  assert_eq!(rows.first().ok_or("missing row")?.n, 1);
  assert_eq!(count_users(&conn).await?, 1);
  Ok(())
}

#[tokio::test]
async fn pool_hands_out_independent_sessions() -> TestResult {
  let (db, first) = pg_schema_conn("conn_pool_multi").await?;
  let second = db.connect().await?;

  // Both sessions are live; the second keeps the default search_path, so it
  // must not see the schema-scoped table created through the first.
  first.execute_batch(USERS_DDL).await?;
  assert_eq!(count_users(&first).await?, 0);
  let visible = second
    .query_map::<CountRow>("SELECT count(*) AS n FROM users", vec![])
    .await;
  let Err(DbError::Query(msg)) = visible else {
    return Err("second session unexpectedly saw the schema-scoped table".into());
  };
  assert!(
    msg.contains("42P01"),
    "expected undefined_table, got: {msg}"
  );
  Ok(())
}

#[tokio::test]
async fn init_fails_fast_when_server_is_unreachable() -> TestResult {
  let config = PgConfig {
    port: 1,
    ..PgConfig::for_test("toolu")
  };
  let Err(err) = PgDatabase::init(&config).await else {
    return Err("init on port 1 unexpectedly succeeded".into());
  };
  assert!(
    matches!(err, DbError::Connection(_)),
    "expected DbError::Connection, got {err:?}"
  );
  assert!(
    err.to_string().contains("initial connection failed"),
    "unexpected message: {err}"
  );
  Ok(())
}
