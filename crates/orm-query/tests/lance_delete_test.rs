//! Real Lance portable DELETE predicates, persistence and affected-row counts.
#![cfg(feature = "lancedb")]

#[path = "fixtures/lance_update.rs"]
pub mod fixture;

use fixture::*;
use toolu_orm_connection::DbError;
use toolu_orm_core::{
  column::Text,
  query_column::{Column, CommonOps},
  value::Value,
};
use toolu_orm_query::{delete::DeleteBuilder, insert::InsertBuilder};

fn original(id: i64) -> Stored {
  Stored(vec![
    Value::Integer(id),
    Value::Text(TEXT.into()),
    Value::Real(-12.5),
    Value::Boolean(true),
    Value::Blob(vec![0, 127, 255]),
    Value::Text("original".into()),
  ])
}

#[tokio::test]
async fn selective_delete_persists_after_reopen() -> TestResult {
  let directory = tempfile::tempdir()?;
  let conn = open(directory.path())?;
  seed(&conn).await?;
  assert_eq!(
    DeleteBuilder::new("insert_source")
      .filter(ID.eq(2_i64))
      .execute_on(&conn)
      .await?,
    1
  );
  drop(conn);
  assert_eq!(rows(directory.path())?, vec![original(1)]);
  Ok(())
}

#[tokio::test]
async fn quoted_values_and_compound_filters_delete_only_exact_matches() -> TestResult {
  let directory = tempfile::tempdir()?;
  let conn = open(directory.path())?;
  seed(&conn).await?;
  let payload = "x' OR 1=1; -- ?1 $2 東京";
  InsertBuilder::new("insert_source")
    .set(&ID, 3_i64)
    .set(&LABEL, payload)
    .set(&SCORE, -12.5)
    .set(&LIVE, true)
    .set(&BYTES, vec![0_u8, 127, 255])
    .set(&NOTE, "original")
    .execute_on(&conn)
    .await?;
  let third = Stored(vec![
    Value::Integer(3),
    Value::Text(payload.into()),
    Value::Real(-12.5),
    Value::Boolean(true),
    Value::Blob(vec![0, 127, 255]),
    Value::Text("original".into()),
  ]);
  let seeded = vec![original(1), original(2), third];
  for absent in ["absent", "absent' OR 1=1 --"] {
    assert_eq!(
      DeleteBuilder::new("insert_source")
        .filter(LABEL.eq(absent))
        .execute_on(&conn)
        .await?,
      0
    );
    assert_eq!(rows(directory.path())?, seeded);
  }
  drop(conn);
  assert_eq!(rows(directory.path())?, seeded);
  let conn = open(directory.path())?;
  assert_eq!(
    DeleteBuilder::new("insert_source")
      .filter(LABEL.eq(payload))
      .filter(ID.eq(1_i64))
      .execute_on(&conn)
      .await?,
    0
  );
  assert_eq!(rows(directory.path())?, seeded);
  assert_eq!(
    DeleteBuilder::new("insert_source")
      .filter(LABEL.eq(payload))
      .execute_on(&conn)
      .await?,
    1
  );
  drop(conn);
  assert_eq!(rows(directory.path())?, vec![original(1), original(2)]);
  let conn = open(directory.path())?;
  assert_eq!(
    DeleteBuilder::new("insert_source")
      .filter(LABEL.eq(TEXT))
      .filter(ID.eq(2_i64))
      .execute_on(&conn)
      .await?,
    1
  );
  drop(conn);
  assert_eq!(rows(directory.path())?, vec![original(1)]);
  Ok(())
}

#[tokio::test]
async fn no_match_multi_match_unfiltered_and_empty_counts_persist() -> TestResult {
  // Exercise multi-match predicates and the intentional no-filter form separately.
  for filtered in [true, false] {
    let directory = tempfile::tempdir()?;
    let conn = open(directory.path())?;
    seed(&conn).await?;
    assert_eq!(
      DeleteBuilder::new("insert_source")
        .filter(ID.eq(999_i64))
        .execute_on(&conn)
        .await?,
      0
    );
    drop(conn);
    assert_eq!(rows(directory.path())?, vec![original(1), original(2)]);
    let conn = open(directory.path())?;
    let statement = DeleteBuilder::new("insert_source");
    let statement = if filtered {
      statement.filter(NOTE.eq("original"))
    } else {
      statement
    };
    assert_eq!(statement.execute_on(&conn).await?, 2);
    drop(conn);
    assert!(rows(directory.path())?.is_empty());
    let conn = open(directory.path())?;
    assert_eq!(statement.execute_on(&conn).await?, 0);
    drop(conn);
    assert!(rows(directory.path())?.is_empty());
  }
  Ok(())
}

#[tokio::test]
async fn invalid_deletes_preserve_rows_and_session_recovers() -> TestResult {
  let directory = tempfile::tempdir()?;
  let conn = open(directory.path())?;
  seed(&conn).await?;
  let missing: Column<Text> = Column::new("insert_source", "missing");
  for statement in [
    DeleteBuilder::new("missing_table").filter(ID.eq(1_i64)),
    DeleteBuilder::new("insert_source").filter(missing.eq("bad")),
    DeleteBuilder::new("insert_source").filter(LABEL.eq(Value::Uuid("deferred".into()))),
  ] {
    assert!(
      matches!(statement.execute_on(&conn).await, Err(DbError::Query(message)) if !message.is_empty())
    );
    assert_eq!(rows(directory.path())?, vec![original(1), original(2)]);
  }
  assert_eq!(
    DeleteBuilder::new("insert_source")
      .filter(ID.eq(2_i64))
      .execute_on(&conn)
      .await?,
    1
  );
  drop(conn);
  assert_eq!(rows(directory.path())?, vec![original(1)]);
  Ok(())
}
