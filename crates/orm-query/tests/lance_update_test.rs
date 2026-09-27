//! Real Lance portable UPDATE persistence and affected-row contract.
#![cfg(feature = "lancedb")]

#[path = "fixtures/lance_update.rs"]
pub mod fixture;

use fixture::*;
use toolu_orm_connection::DbError;
use toolu_orm_core::{
  column::Text,
  expr::Scalar,
  query_column::{Column, CommonOps},
  value::Value,
};
use toolu_orm_query::update::UpdateBuilder;

fn stored(id: i64, label: &str, score: f64, note: Value) -> Stored {
  Stored(vec![
    Value::Integer(id),
    Value::Text(label.into()),
    Value::Real(score),
    Value::Boolean(true),
    Value::Blob(vec![0, 127, 255]),
    note,
  ])
}

fn original(id: i64) -> Stored {
  stored(id, TEXT, -12.5, Value::Text("original".into()))
}

#[tokio::test]
async fn selective_scalar_assignments_persist_after_reopen() -> TestResult {
  let directory = tempfile::tempdir()?;
  let conn = open(directory.path())?;
  seed(&conn).await?;
  let label = "updated O'Brien ?1 $7; -- 東京";
  assert_eq!(
    UpdateBuilder::new("insert_source")
      .set(&LABEL, label)
      .set(&SCORE, 8.25)
      .set(&LIVE, false)
      .set(&NOTE, Value::Null)
      .filter(ID.eq(2_i64))
      .execute_on(&conn)
      .await?,
    1
  );
  let changed = Stored(vec![
    Value::Integer(2),
    Value::Text(label.into()),
    Value::Real(8.25),
    Value::Boolean(false),
    Value::Blob(vec![0, 127, 255]),
    Value::Null,
  ]);
  let expected = vec![original(1), changed];
  assert_eq!(rows(directory.path())?, expected);
  drop(conn);
  assert_eq!(rows(directory.path())?, expected);
  Ok(())
}

#[tokio::test]
async fn computed_and_raw_assignments_bind_before_typed_filters() -> TestResult {
  let directory = tempfile::tempdir()?;
  let conn = open(directory.path())?;
  seed(&conn).await?;
  assert_eq!(
    UpdateBuilder::new("insert_source")
      .set_scalar(&SCORE, Scalar::col(&SCORE) + Scalar::bind(20.0))
      .set(&LABEL, "computed ?3 $4")
      .filter(ID.eq(2_i64))
      .filter(SCORE.eq(-12.5))
      .execute_on(&conn)
      .await?,
    1
  );
  drop(conn);
  let conn = open(directory.path())?;
  assert_eq!(
    rows(directory.path())?,
    vec![
      original(1),
      stored(2, "computed ?3 $4", 7.5, Value::Text("original".into()))
    ]
  );
  assert_eq!(
    UpdateBuilder::new("insert_source")
      .set_expr(&SCORE, "score * 2")
      .filter(ID.eq(2_i64))
      .execute_on(&conn)
      .await?,
    1
  );
  drop(conn);
  assert_eq!(
    rows(directory.path())?,
    vec![
      original(1),
      stored(2, "computed ?3 $4", 15.0, Value::Text("original".into()))
    ]
  );
  Ok(())
}

#[tokio::test]
async fn no_match_same_value_and_unfiltered_counts_persist() -> TestResult {
  let directory = tempfile::tempdir()?;
  let conn = open(directory.path())?;
  seed(&conn).await?;
  assert_eq!(
    UpdateBuilder::new("insert_source")
      .set(&LABEL, "absent")
      .filter(ID.eq(999_i64))
      .execute_on(&conn)
      .await?,
    0
  );
  drop(conn);
  let conn = open(directory.path())?;
  assert_eq!(rows(directory.path())?, vec![original(1), original(2)]);
  assert_eq!(
    UpdateBuilder::new("insert_source")
      .set(&LABEL, TEXT)
      .filter(ID.eq(1_i64))
      .execute_on(&conn)
      .await?,
    1
  );
  assert_eq!(rows(directory.path())?, vec![original(1), original(2)]);
  assert_eq!(
    UpdateBuilder::new("insert_source")
      .set(&LABEL, "")
      .execute_on(&conn)
      .await?,
    2
  );
  drop(conn);
  assert_eq!(
    rows(directory.path())?,
    vec![
      stored(1, "", -12.5, Value::Text("original".into())),
      stored(2, "", -12.5, Value::Text("original".into()))
    ]
  );
  Ok(())
}

#[tokio::test]
async fn invalid_updates_preserve_rows_and_session_recovers() -> TestResult {
  let directory = tempfile::tempdir()?;
  let conn = open(directory.path())?;
  seed(&conn).await?;
  let missing: Column<Text> = Column::new("insert_source", "missing");
  for bytes in [vec![], vec![9_u8, 0, 255]] {
    let result = UpdateBuilder::new("insert_source")
      .set(&LABEL, "must not partially write")
      .set(&BYTES, bytes)
      .filter(ID.eq(2_i64))
      .execute_on(&conn)
      .await;
    assert!(
      matches!(result, Err(DbError::Query(message)) if message.contains("Lance UPDATE does not support literal type BLOB"))
    );
    assert_eq!(rows(directory.path())?, vec![original(1), original(2)]);
  }
  for statement in [
    UpdateBuilder::new("missing_table").set(&LABEL, "bad"),
    UpdateBuilder::new("insert_source").set(&missing, "bad"),
    UpdateBuilder::new("insert_source").filter(ID.eq(1_i64)),
    UpdateBuilder::new("insert_source").set(&LABEL, Value::Uuid("deferred".into())),
  ] {
    assert!(
      matches!(statement.execute_on(&conn).await, Err(DbError::Query(message)) if !message.is_empty())
    );
    assert_eq!(rows(directory.path())?, vec![original(1), original(2)]);
  }
  assert_eq!(
    UpdateBuilder::new("insert_source")
      .set(&NOTE, "recovered")
      .filter(ID.eq(2_i64))
      .execute_on(&conn)
      .await?,
    1
  );
  drop(conn);
  assert_eq!(
    rows(directory.path())?,
    vec![
      original(1),
      stored(2, TEXT, -12.5, Value::Text("recovered".into()))
    ]
  );
  Ok(())
}
