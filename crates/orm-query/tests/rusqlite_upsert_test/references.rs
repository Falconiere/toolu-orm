//! Foreign-key safety: the difference between updating the conflicting row
//! and deleting it, proven with a real `ON DELETE CASCADE` child row.

use toolu_orm_core::expr::Scalar;
use toolu_orm_query::insert::{InsertBuilder, OnConflict};

use super::db;
use super::schema::{BODY, MEMORY_ID, USED_COUNT};
use super::support::{links, memory, seed_link, seed_memory, TestResult};

#[test]
fn the_fixture_really_enforces_foreign_keys() -> TestResult {
  let conn = db::setup_db()?;
  assert_eq!(
    db::foreign_keys_enabled(&conn)?,
    1,
    "the cascade contrast below is meaningless with the pragma off"
  );
  Ok(())
}

#[test]
fn an_upsert_keeps_the_rows_that_reference_the_conflicting_row() -> TestResult {
  let conn = db::setup_db()?;
  seed_memory(&conn)?;
  seed_link(&conn, "l1", "m1")?;

  InsertBuilder::new("memories")
    .set(&MEMORY_ID, "m1")
    .set(&BODY, "incoming")
    .on_conflict(OnConflict::column(&MEMORY_ID).set_excluded(&BODY))
    .execute(&conn)?;

  let surviving = links(&conn)?;
  assert_eq!(surviving.len(), 1, "DO UPDATE never deletes the parent row");
  assert_eq!(surviving.first().map(|link| link.id.as_str()), Some("l1"));
  assert_eq!(memory(&conn, "m1")?.body, "incoming");
  Ok(())
}

#[test]
fn or_replace_cascades_the_referencing_row_away() -> TestResult {
  let conn = db::setup_db()?;
  seed_memory(&conn)?;
  seed_link(&conn, "l1", "m1")?;

  InsertBuilder::new("memories")
    .set(&MEMORY_ID, "m1")
    .set(&BODY, "incoming")
    .or_replace()
    .execute(&conn)?;

  assert!(
    links(&conn)?.is_empty(),
    "OR REPLACE deletes and re-inserts, so ON DELETE CASCADE fires"
  );
  let row = memory(&conn, "m1")?;
  assert_eq!(
    row.used_count, 0,
    "and every column the insert did not name falls back to its default"
  );
  assert_eq!(row.workspace_id, None);
  Ok(())
}

#[test]
fn an_upsert_that_raises_the_counter_still_keeps_the_child_row() -> TestResult {
  let conn = db::setup_db()?;
  seed_memory(&conn)?;
  seed_link(&conn, "l1", "m1")?;

  InsertBuilder::new("memories")
    .set(&MEMORY_ID, "m1")
    .set(&BODY, "incoming")
    .set(&USED_COUNT, 1_i64)
    .on_conflict(
      OnConflict::column(&MEMORY_ID)
        .set_scalar(&USED_COUNT, Scalar::col(&USED_COUNT) + Scalar::bind(1_i64)),
    )
    .execute(&conn)?;

  assert_eq!(links(&conn)?.len(), 1);
  assert_eq!(memory(&conn, "m1")?.used_count, 4);
  Ok(())
}
