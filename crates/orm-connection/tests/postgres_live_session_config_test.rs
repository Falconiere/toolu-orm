//! `PgTransaction::set_local_config` on a live Postgres: the setting is
//! visible for the rest of the transaction, gone once it ends, and a name
//! Postgres rejects surfaces as a query error rather than SQL text.
//!
//! Needs `docker compose -f docker-compose.test.yaml up -d --wait` and
//! `TEST_DB_PORT=5434` locally. Compiles only on the five-crate postgres lane.
#![cfg(all(feature = "libsql", feature = "postgres"))]

#[path = "fixtures/pg_live.rs"]
mod pg_live;

use toolu_orm_connection::{DbConnection, DbError};
use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::row::FromRow;
use toolu_orm_core::value::Value;

use pg_live::{TestResult, USERS_DDL, count_users, pg_schema_conn};

struct SettingRow {
  value: String,
}

impl FromRow for SettingRow {
  const REQUIRED_COLUMNS: &'static [&'static str] = &["value"];

  fn from_pg_row(row: &tokio_postgres::Row) -> Result<Self, DbCoreError> {
    let value = row
      .try_get::<usize, String>(0)
      .map_err(|e| DbCoreError::RowMapping(format!("column 0 (value): {e}")))?;
    Ok(Self { value })
  }

  fn from_libsql_row(_: &libsql::Row) -> Result<Self, DbCoreError> {
    Err(DbCoreError::RowMapping(
      "SettingRow is only decoded from Postgres rows".into(),
    ))
  }
}

/// `current_setting(name, true)`: the empty string when the setting is unset.
async fn read_setting(conn: &impl DbConnection, name: &str) -> Result<String, DbError> {
  let rows = conn
    .query_map::<SettingRow>(
      "SELECT current_setting($1, true) AS value",
      vec![Value::Text(name.to_owned())],
    )
    .await?;
  Ok(rows.into_iter().next().map(|r| r.value).unwrap_or_default())
}

#[tokio::test]
async fn setting_is_visible_inside_and_reset_after_the_transaction() -> TestResult {
  let (_db, mut conn) = pg_schema_conn("conn_session_config").await?;
  conn.execute_batch(USERS_DDL).await?;

  let tx = conn.transaction().await?;
  tx.set_local_config("app.tenant_id", "42").await?;
  assert_eq!(read_setting(&tx, "app.tenant_id").await?, "42");
  tx.commit().await?;

  assert_eq!(read_setting(&conn, "app.tenant_id").await?, "");
  assert_eq!(count_users(&conn).await?, 0);
  Ok(())
}

#[tokio::test]
async fn value_is_bound_not_interpolated() -> TestResult {
  let (_db, mut conn) = pg_schema_conn("conn_session_config_bound").await?;
  conn.execute_batch(USERS_DDL).await?;

  let hostile = "1'; INSERT INTO users (id, name) VALUES ('x', 'y'); --";
  let tx = conn.transaction().await?;
  tx.set_local_config("app.tenant_id", hostile).await?;
  assert_eq!(read_setting(&tx, "app.tenant_id").await?, hostile);
  assert_eq!(count_users(&tx).await?, 0);
  tx.commit().await?;
  assert_eq!(count_users(&conn).await?, 0);
  Ok(())
}

#[tokio::test]
async fn rejected_name_is_a_query_error() -> TestResult {
  let (_db, mut conn) = pg_schema_conn("conn_session_config_reject").await?;
  let tx = conn.transaction().await?;
  let err = tx
    .set_local_config("no_dot_in_name", "1")
    .await
    .err()
    .ok_or("a one-part custom name should be rejected")?;
  assert!(
    matches!(&err, DbError::Query(msg) if msg.contains("unrecognized configuration parameter") && msg.contains("42704")),
    "err: {err}"
  );
  Ok(())
}
