//! `#[derive(FromRow)]` against real rows.
//!
//! The derive decodes Postgres rows positionally via `try_get(idx)`; its libsql
//! method is a documented error stub (`orm-macros/src/from_row_expand.rs`).
//! Needs the live Postgres from `docker-compose.test.yaml`.
#![cfg(all(feature = "libsql", feature = "postgres"))]

use toolu_orm_connection::{Database, DbConnection, DbError, PgConfig, PgDatabase};
use toolu_orm_core::value::Value;
use toolu_orm_macros::FromRow;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[derive(FromRow, Debug, PartialEq)]
struct Person {
  id: String,
  age: Option<i64>,
}

const DDL: &str = "CREATE TABLE people (id TEXT PRIMARY KEY, age BIGINT)";
const INSERT: &str = "INSERT INTO people (id, age) VALUES ($1, $2)";

async fn pg_people(
  schema: &str,
) -> Result<(PgDatabase, impl DbConnection), Box<dyn std::error::Error>> {
  let db = PgDatabase::init(&PgConfig::for_test("toolu")).await?;
  let conn = db.connect().await?;
  conn
    .execute_batch(&format!(
      "DROP SCHEMA IF EXISTS {schema} CASCADE; CREATE SCHEMA {schema}; SET search_path TO {schema}; {DDL}"
    ))
    .await?;
  conn
    .execute_sql(INSERT, vec![Value::Text("p1".into()), Value::Null])
    .await?;
  conn
    .execute_sql(INSERT, vec![Value::Text("p2".into()), Value::Integer(30)])
    .await?;
  Ok((db, conn))
}

#[tokio::test]
async fn derive_decodes_postgres_rows_with_null_as_none() -> TestResult {
  let (_db, conn) = pg_people("conn_derive_null").await?;
  let people = conn
    .query_map::<Person>("SELECT id, age FROM people ORDER BY id", vec![])
    .await?;
  assert_eq!(
    people,
    vec![
      Person {
        id: "p1".into(),
        age: None
      },
      Person {
        id: "p2".into(),
        age: Some(30)
      },
    ]
  );
  Ok(())
}

#[tokio::test]
async fn derive_fewer_columns_than_required_is_row_mapping() -> TestResult {
  let (_db, conn) = pg_people("conn_derive_fewer").await?;
  let result = conn
    .query_map::<Person>("SELECT id FROM people", vec![])
    .await;
  let Err(DbError::RowMapping(msg)) = result else {
    return Err("expected DbError::RowMapping".into());
  };
  assert!(msg.contains("column 1 (age)"), "unexpected message: {msg}");
  Ok(())
}

#[tokio::test]
async fn derive_on_libsql_row_returns_documented_stub_error() -> TestResult {
  let db = Database::init_local(":memory:").await?;
  let conn = db.connect()?;
  conn
    .execute_batch("CREATE TABLE people (id TEXT PRIMARY KEY, age INTEGER)")
    .await?;
  conn
    .execute_sql(
      "INSERT INTO people (id, age) VALUES (?1, ?2)",
      vec![Value::Text("p1".into()), Value::Integer(30)],
    )
    .await?;
  let result = conn
    .query_map::<Person>("SELECT id, age FROM people", vec![])
    .await;
  let Err(DbError::RowMapping(msg)) = result else {
    return Err("expected DbError::RowMapping".into());
  };
  assert!(
    msg.ends_with("is only decoded from Postgres rows"),
    "unexpected message: {msg}"
  );
  Ok(())
}
