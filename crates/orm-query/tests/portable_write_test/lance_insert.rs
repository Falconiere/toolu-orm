use super::{lance_insert_support::*, support::TestResult};
use toolu_orm_connection::DbError;
use toolu_orm_core::{expr::Scalar, query_column::NumericOps, value::Value};
use toolu_orm_query::{insert::InsertBuilder, select::SelectBuilder};

fn scalar_row(id: i64, empty: bool) -> InsertBuilder {
  InsertBuilder::new("insert_source")
    .set(&ID, id)
    .set(&LABEL, if empty { "" } else { TEXT })
    .set(&SCORE, if empty { 0.0 } else { -12.5 })
    .set(&LIVE, !empty)
    .set(&BYTES, if empty { vec![] } else { vec![0_u8, 127, 255] })
    .set_null(&NOTE)
}

fn copy_rows(threshold: i64) -> InsertBuilder {
  let source = SelectBuilder::new("insert_source")
    .columns_raw(&["id", "label", "score", "live", "bytes"])
    .column_scalar(Scalar::bind("copied ?1 $2 東京"), "note")
    .filter(ID.gt(threshold));
  InsertBuilder::new("insert_target").select(&[&ID, &LABEL, &SCORE, &LIVE, &BYTES, &NOTE], source)
}

#[tokio::test]
async fn bound_scalar_rows_persist_after_reopen() -> TestResult {
  let directory = tempfile::tempdir()?;
  let conn = open(directory.path())?;
  setup(&conn).await?;
  assert_eq!(scalar_row(-7, false).execute_on(&conn).await?, 1);
  assert_eq!(rows(&conn, false).await?, vec![expected(-7, false)]);
  drop(conn);
  let conn = open(directory.path())?;
  assert_eq!(rows(&conn, false).await?, vec![expected(-7, false)]);
  for (id, empty) in [(0, true), (42, false)] {
    assert_eq!(scalar_row(id, empty).execute_on(&conn).await?, 1);
  }
  drop(conn);
  let conn = open(directory.path())?;
  assert_eq!(
    rows(&conn, false).await?,
    vec![expected(-7, false), expected(0, true), expected(42, false)]
  );
  Ok(())
}

#[tokio::test]
async fn insert_select_copies_projection_and_empty_source_persists() -> TestResult {
  let directory = tempfile::tempdir()?;
  let conn = open(directory.path())?;
  setup(&conn).await?;
  for (id, empty) in [(-7, false), (0, true), (42, false)] {
    scalar_row(id, empty).execute_on(&conn).await?;
  }
  // One execute_on sends INSERT SELECT; the source is never fetched into Rust.
  assert_eq!(copy_rows(-1).execute_on(&conn).await?, 2);
  let projected = |id, empty| {
    let mut row = expected(id, empty);
    if let Some(note) = row.0.last_mut() {
      *note = Value::Text("copied ?1 $2 東京".into());
    }
    row
  };
  let expected_rows = vec![projected(0, true), projected(42, false)];
  assert_eq!(rows(&conn, true).await?, expected_rows);
  assert_eq!(copy_rows(100).execute_on(&conn).await?, 0);
  assert_eq!(rows(&conn, true).await?, expected_rows);
  drop(conn);
  let conn = open(directory.path())?;
  assert_eq!(rows(&conn, true).await?, expected_rows);
  assert_eq!(
    rows(&conn, false).await?,
    vec![expected(-7, false), expected(0, true), expected(42, false)]
  );
  Ok(())
}

#[tokio::test]
async fn invalid_insert_preserves_rows_and_session_recovers() -> TestResult {
  let directory = tempfile::tempdir()?;
  let conn = open(directory.path())?;
  setup(&conn).await?;
  scalar_row(1, false).execute_on(&conn).await?;
  let invalid = [
    InsertBuilder::new("missing_target").set(&ID, 2_i64),
    InsertBuilder::new("insert_source").select_raw(
      COLUMNS,
      SelectBuilder::new("missing_source").columns_raw(COLUMNS),
    ),
    InsertBuilder::new("insert_source").select_raw(
      COLUMNS,
      SelectBuilder::new("insert_source").columns_raw(&["id"]),
    ),
    InsertBuilder::new("insert_source")
      .set(&ID, 2_i64)
      .set(&LABEL, Value::Uuid("deferred codec".into())),
  ];
  for statement in invalid {
    assert!(
      matches!(statement.execute_on(&conn).await, Err(DbError::Query(message)) if !message.is_empty())
    );
    assert_eq!(rows(&conn, false).await?, vec![expected(1, false)]);
  }
  assert_eq!(scalar_row(2, true).execute_on(&conn).await?, 1);
  drop(conn);
  let conn = open(directory.path())?;
  assert_eq!(
    rows(&conn, false).await?,
    vec![expected(1, false), expected(2, true)]
  );
  Ok(())
}
