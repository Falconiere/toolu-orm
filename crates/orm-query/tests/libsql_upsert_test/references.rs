//! Foreign-key safety: the difference between updating the conflicting row
//! and deleting it, proven with a real `ON DELETE CASCADE` child row.

use toolu_orm_query::insert::{InsertBuilder, OnConflict};

use super::db;
use super::schema::{BODY, MEMORY_ID};
use super::support::{links, memory, seed_link, seed_memory, TestResult};

#[tokio::test]
async fn the_fixture_really_enforces_foreign_keys() -> TestResult {
  let conn = db::setup_db().await?;
  assert_eq!(
    db::foreign_keys_enabled(&conn).await?,
    1,
    "the cascade contrast below is meaningless with the pragma off"
  );
  Ok(())
}

#[tokio::test]
async fn an_upsert_keeps_the_rows_that_reference_the_conflicting_row() -> TestResult {
  let conn = db::setup_db().await?;
  seed_memory(&conn).await?;
  seed_link(&conn, "l1", "m1").await?;

  InsertBuilder::new("memories")
    .set(&MEMORY_ID, "m1")
    .set(&BODY, "incoming")
    .on_conflict(OnConflict::column(&MEMORY_ID).set_excluded(&BODY))
    .execute(&conn)
    .await?;

  let surviving = links(&conn).await?;
  assert_eq!(surviving.len(), 1, "DO UPDATE never deletes the parent row");
  assert_eq!(surviving.first().map(|link| link.id.as_str()), Some("l1"));
  assert_eq!(memory(&conn, "m1").await?.body, "incoming");
  Ok(())
}

#[tokio::test]
async fn or_replace_cascades_the_referencing_row_away() -> TestResult {
  let conn = db::setup_db().await?;
  seed_memory(&conn).await?;
  seed_link(&conn, "l1", "m1").await?;

  InsertBuilder::new("memories")
    .set(&MEMORY_ID, "m1")
    .set(&BODY, "incoming")
    .or_replace()
    .execute(&conn)
    .await?;

  assert!(
    links(&conn).await?.is_empty(),
    "OR REPLACE deletes and re-inserts, so ON DELETE CASCADE fires"
  );
  let row = memory(&conn, "m1").await?;
  assert_eq!(
    row.used_count, 0,
    "and every column the insert did not name falls back to its default"
  );
  assert_eq!(row.workspace_id, None);
  Ok(())
}
