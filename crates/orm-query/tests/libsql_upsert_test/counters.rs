//! Counter increments and partial-field preservation: what `DO UPDATE` writes
//! and, more to the point, what it leaves alone.

use toolu_orm_core::expr::Scalar;
use toolu_orm_query::insert::{InsertBuilder, OnConflict};

use super::db;
use super::schema::{BODY, LAST_USED, MEMORY_ID, USED_COUNT, WORKSPACE_ID};
use super::support::{memory, seed_memory, TestResult};

/// The `feedback.rs` statement from issue #108: bump the counter, refresh the
/// timestamp, name nothing else.
fn bump(stamp: &str) -> InsertBuilder {
  InsertBuilder::new("memories")
    .set(&MEMORY_ID, "m1")
    .set(&BODY, "incoming")
    .set(&USED_COUNT, 1_i64)
    .set(&LAST_USED, stamp)
    .on_conflict(
      OnConflict::column(&MEMORY_ID)
        .set_scalar(&USED_COUNT, Scalar::col(&USED_COUNT) + Scalar::bind(1_i64))
        .set(&LAST_USED, stamp),
    )
}

#[tokio::test]
async fn the_first_insert_writes_the_values_rather_than_the_update() -> TestResult {
  let conn = db::setup_db().await?;
  assert_eq!(bump("t1").execute(&conn).await?, 1);

  let row = memory(&conn, "m1").await?;
  assert_eq!(row.used_count, 1, "no conflict, so no increment");
  assert_eq!(row.body, "incoming");
  assert_eq!(row.last_used.as_deref(), Some("t1"));
  Ok(())
}

#[tokio::test]
async fn a_conflict_increments_the_counter_and_leaves_unnamed_columns_alone() -> TestResult {
  let conn = db::setup_db().await?;
  seed_memory(&conn).await?;

  assert_eq!(bump("t1").execute(&conn).await?, 1);

  let row = memory(&conn, "m1").await?;
  assert_eq!(row.used_count, 4, "3 + 1, read from the stored row");
  assert_eq!(row.last_used.as_deref(), Some("t1"));
  assert_eq!(row.body, "original", "body is not in the DO UPDATE list");
  assert_eq!(
    row.workspace_id.as_deref(),
    Some("w1"),
    "workspace_id is not in the DO UPDATE list"
  );
  Ok(())
}

#[tokio::test]
async fn do_nothing_reports_no_affected_row_and_changes_nothing() -> TestResult {
  let conn = db::setup_db().await?;
  seed_memory(&conn).await?;
  let before = memory(&conn, "m1").await?;

  let affected = InsertBuilder::new("memories")
    .set(&MEMORY_ID, "m1")
    .set(&BODY, "incoming")
    .set(&USED_COUNT, 99_i64)
    .on_conflict(OnConflict::column(&MEMORY_ID).do_nothing())
    .execute(&conn)
    .await?;

  assert_eq!(affected, 0);
  assert_eq!(memory(&conn, "m1").await?, before);
  Ok(())
}

#[tokio::test]
async fn coalesce_keeps_a_stored_workspace_and_adopts_a_missing_one() -> TestResult {
  let conn = db::setup_db().await?;
  seed_memory(&conn).await?;

  let keep_stored = Scalar::func(
    "coalesce",
    vec![Scalar::col(&WORKSPACE_ID), Scalar::excluded(&WORKSPACE_ID)],
  )?;
  InsertBuilder::new("memories")
    .set(&MEMORY_ID, "m1")
    .set(&BODY, "incoming")
    .set_null(&WORKSPACE_ID)
    .on_conflict(
      OnConflict::column(&MEMORY_ID)
        .set_excluded(&BODY)
        .set_scalar(&WORKSPACE_ID, keep_stored),
    )
    .execute(&conn)
    .await?;

  let kept = memory(&conn, "m1").await?;
  assert_eq!(kept.body, "incoming", "excluded.body was taken");
  assert_eq!(
    kept.workspace_id.as_deref(),
    Some("w1"),
    "the incoming NULL must not overwrite the stored workspace"
  );

  let adopt = Scalar::func(
    "coalesce",
    vec![Scalar::excluded(&WORKSPACE_ID), Scalar::col(&WORKSPACE_ID)],
  )?;
  InsertBuilder::new("memories")
    .set(&MEMORY_ID, "m1")
    .set(&BODY, "incoming")
    .set(&WORKSPACE_ID, "w2")
    .on_conflict(OnConflict::column(&MEMORY_ID).set_scalar(&WORKSPACE_ID, adopt))
    .execute(&conn)
    .await?;
  assert_eq!(
    memory(&conn, "m1").await?.workspace_id.as_deref(),
    Some("w2")
  );
  Ok(())
}
