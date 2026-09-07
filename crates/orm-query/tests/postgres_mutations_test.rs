//! Executed `InsertBuilder` / `UpdateBuilder` / `DeleteBuilder` scenarios on a
//! live Postgres: CRUD round trip, `ON CONFLICT` upsert modes, and every
//! `Value` variant bound and read back.
//!
//! Needs `docker compose -f docker-compose.test.yaml up -d --wait` and
//! `TEST_DB_PORT=5434` locally; compiles only via the five-crate postgres lane.

#[path = "fixtures/postgres_db.rs"]
pub mod pg;

use toolu_orm_core::libsql;
use toolu_orm_core::query_column::CommonOps;
use toolu_orm_core::value::Value;
use toolu_orm_macros::FromRow;
use toolu_orm_query::delete::DeleteBuilder;
use toolu_orm_query::executor::Executor;
use toolu_orm_query::insert::InsertBuilder;
use toolu_orm_query::update::UpdateBuilder;

use pg::{all_users, client, insert_user, TestResult, User, AGE, EMAIL, ID, NAME};

#[tokio::test]
async fn insert_update_delete_round_trip() -> TestResult {
  let client = client("q_pg_crud").await?;
  assert_eq!(
    insert_user(&client, "u1", "Ann", "ann@x.io", Some(30)).await?,
    1
  );
  assert_eq!(
    all_users(&client).await?,
    vec![User {
      id: "u1".into(),
      name: "Ann".into(),
      email: "ann@x.io".into(),
      age: Some(30)
    }]
  );

  let updated = UpdateBuilder::new("users")
    .set(&NAME, "Ann B")
    .set_expr(&AGE, "\"age\" + 1")
    .filter(ID.eq("u1"))
    .execute(&client)
    .await?;
  assert_eq!(updated, 1);
  let user = all_users(&client)
    .await?
    .into_iter()
    .next()
    .ok_or("row gone")?;
  assert_eq!((user.name.as_str(), user.age), ("Ann B", Some(31)));

  let deleted = DeleteBuilder::new("users")
    .filter(ID.eq("u1"))
    .execute(&client)
    .await?;
  assert_eq!(deleted, 1);
  assert!(all_users(&client).await?.is_empty());
  Ok(())
}

#[tokio::test]
async fn or_replace_updates_the_conflicting_row() -> TestResult {
  let client = client("q_pg_upsert_replace").await?;
  insert_user(&client, "u1", "Ann", "ann@x.io", None).await?;
  let affected = InsertBuilder::new("users")
    .set(&ID, "u1")
    .set(&NAME, "Bea")
    .set(&EMAIL, "bea@x.io")
    .or_replace()
    .conflict_columns(&["id"])
    .execute(&client)
    .await?;
  assert_eq!(affected, 1);
  let users = all_users(&client).await?;
  assert_eq!(users.len(), 1);
  assert_eq!(users.first().map(|u| u.name.as_str()), Some("Bea"));
  Ok(())
}

#[tokio::test]
async fn or_ignore_keeps_the_existing_row() -> TestResult {
  let client = client("q_pg_upsert_ignore").await?;
  insert_user(&client, "u1", "Ann", "ann@x.io", None).await?;
  let affected = InsertBuilder::new("users")
    .set(&ID, "u1")
    .set(&NAME, "Bea")
    .set(&EMAIL, "bea@x.io")
    .or_ignore()
    .execute(&client)
    .await?;
  assert_eq!(affected, 0, "ON CONFLICT DO NOTHING must not touch the row");
  let users = all_users(&client).await?;
  assert_eq!(users.len(), 1);
  assert_eq!(users.first().map(|u| u.name.as_str()), Some("Ann"));
  Ok(())
}

#[derive(FromRow, Debug, PartialEq)]
struct ValueRow {
  id: i64,
  t: String,
  r: f64,
  b: Vec<u8>,
  n: Option<i64>,
}

#[tokio::test]
async fn every_value_variant_binds_and_reads_back() -> TestResult {
  let client = client("q_pg_values").await?;
  client
    .batch_execute(
      "CREATE TABLE v (id BIGINT PRIMARY KEY, t TEXT, r DOUBLE PRECISION, b BYTEA, n BIGINT)",
    )
    .await?;
  let inserted = client
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
  assert_eq!(inserted, 1);

  let rows = client
    .query_map::<ValueRow>(
      "SELECT id, t, r, b, n FROM v WHERE id = $1 AND t = $2 AND r = $3 AND b = $4 AND n IS NULL",
      vec![
        Value::Integer(i64::MAX),
        Value::Text("héllo".into()),
        Value::Real(1.5),
        Value::Blob(vec![0, 255, 7]),
      ],
    )
    .await?;
  assert_eq!(
    rows,
    vec![ValueRow {
      id: i64::MAX,
      t: "héllo".into(),
      r: 1.5,
      b: vec![0, 255, 7],
      n: None
    }]
  );
  Ok(())
}
