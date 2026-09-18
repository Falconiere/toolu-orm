//! Issue #108's conflict clause against live Postgres: the very same builder
//! chain the SQLite suites run, proving the clause is explicit on both
//! engines rather than a SQLite form approximated for Postgres.
//!
//! Needs the `docker-compose.test.yaml` server; a missing one fails the test.

#[path = "fixtures/pg_upsert_db.rs"]
pub mod db;
#[path = "fixtures/upsert_schema.rs"]
pub mod schema;

use toolu_orm_core::expr::Scalar;
use toolu_orm_core::query_column::CommonOps;
use toolu_orm_query::insert::{InsertBuilder, OnConflict};
use toolu_orm_query::select::SelectBuilder;
use toolu_orm_query::QueryError;

use schema::{
  CodeSymbol, GeneratedId, Memory, MemoryId, MemoryLink, BODY, INDEXED_AT, LAST_USED, LINK_COLUMNS,
  LINK_ID, LINK_MEMORY_ID, LINK_NOTE, MEMORY_COLUMNS, MEMORY_ID, PATH, REPO, SYMBOL_COLUMNS,
  SYMBOL_ID, USED_COUNT, WORKSPACE_ID,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

async fn seed_memory(client: &tokio_postgres::Client) -> TestResult {
  InsertBuilder::new("memories")
    .set(&MEMORY_ID, "m1")
    .set(&BODY, "original")
    .set(&WORKSPACE_ID, "w1")
    .set(&USED_COUNT, 3_i64)
    .set(&LAST_USED, "t0")
    .execute(client)
    .await?;
  Ok(())
}

async fn memory(client: &tokio_postgres::Client) -> Result<Memory, QueryError> {
  SelectBuilder::new("memories")
    .columns_raw(&MEMORY_COLUMNS)
    .filter(MEMORY_ID.eq("m1"))
    .fetch_one(client)
    .await
}

#[tokio::test]
async fn a_conflict_increments_the_counter_and_leaves_unnamed_columns_alone() -> TestResult {
  let client = db::setup_db("upsert_counter").await?;
  seed_memory(&client).await?;

  let affected = InsertBuilder::new("memories")
    .set(&MEMORY_ID, "m1")
    .set(&BODY, "incoming")
    .set(&USED_COUNT, 1_i64)
    .set(&LAST_USED, "t1")
    .on_conflict(
      OnConflict::column(&MEMORY_ID)
        .set_scalar(&USED_COUNT, Scalar::col(&USED_COUNT) + Scalar::bind(1_i64))
        .set(&LAST_USED, "t1"),
    )
    .execute(&client)
    .await?;

  assert_eq!(affected, 1);
  let row = memory(&client).await?;
  assert_eq!(row.used_count, 4, "3 + 1, read from the stored row");
  assert_eq!(row.last_used.as_deref(), Some("t1"));
  assert_eq!(row.body, "original", "body is not in the DO UPDATE list");
  assert_eq!(row.workspace_id.as_deref(), Some("w1"));
  Ok(())
}

#[tokio::test]
async fn do_nothing_reports_no_affected_row_and_changes_nothing() -> TestResult {
  let client = db::setup_db("upsert_do_nothing").await?;
  seed_memory(&client).await?;
  let before = memory(&client).await?;

  let affected = InsertBuilder::new("memories")
    .set(&MEMORY_ID, "m1")
    .set(&BODY, "incoming")
    .on_conflict(OnConflict::column(&MEMORY_ID).do_nothing())
    .execute(&client)
    .await?;

  assert_eq!(affected, 0);
  assert_eq!(memory(&client).await?, before);
  Ok(())
}

#[tokio::test]
async fn excluded_and_coalesce_preserve_the_stored_workspace() -> TestResult {
  let client = db::setup_db("upsert_excluded").await?;
  seed_memory(&client).await?;

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
    .execute(&client)
    .await?;

  let row = memory(&client).await?;
  assert_eq!(row.body, "incoming", "excluded.body was taken");
  assert_eq!(
    row.workspace_id.as_deref(),
    Some("w1"),
    "the incoming NULL must not overwrite the stored workspace"
  );
  Ok(())
}

#[tokio::test]
async fn an_upsert_keeps_the_rows_that_reference_the_conflicting_row() -> TestResult {
  let client = db::setup_db("upsert_references").await?;
  seed_memory(&client).await?;
  InsertBuilder::new("memory_links")
    .set(&LINK_ID, "l1")
    .set(&LINK_MEMORY_ID, "m1")
    .set(&LINK_NOTE, "keep me")
    .execute(&client)
    .await?;

  InsertBuilder::new("memories")
    .set(&MEMORY_ID, "m1")
    .set(&BODY, "incoming")
    .on_conflict(OnConflict::column(&MEMORY_ID).set_excluded(&BODY))
    .execute(&client)
    .await?;

  let links: Vec<MemoryLink> = SelectBuilder::new("memory_links")
    .columns_raw(&LINK_COLUMNS)
    .fetch_all(&client)
    .await?;
  assert_eq!(links.len(), 1, "DO UPDATE never deletes the parent row");
  assert_eq!(memory(&client).await?.body, "incoming");
  Ok(())
}

#[tokio::test]
async fn returning_hands_back_the_identity_id_and_keeps_it_across_a_conflict() -> TestResult {
  let client = db::setup_db("upsert_returning").await?;
  let index = |path: &'static str, stamp: &'static str| {
    InsertBuilder::new("code_symbols")
      .set(&REPO, "toolu-orm")
      .set(&PATH, path)
      .set(&INDEXED_AT, stamp)
      .on_conflict(
        OnConflict::column(&REPO)
          .and_column(&PATH)
          .set_excluded(&INDEXED_AT),
      )
      .returning(&SYMBOL_ID)
  };

  let first: GeneratedId = index("src/lib.rs", "t1").fetch_one(&client).await?;
  assert_eq!(first.id, 1);

  let second: GeneratedId = index("src/main.rs", "t1").fetch_one(&client).await?;
  assert_eq!(second.id, 2, "a distinct row takes the next identity value");

  let again: GeneratedId = index("src/lib.rs", "t2").fetch_one(&client).await?;
  assert_eq!(
    again.id, first.id,
    "DO UPDATE writes the stored row, so its id survives"
  );

  let rows: Vec<CodeSymbol> = SelectBuilder::new("code_symbols")
    .columns_raw(&SYMBOL_COLUMNS)
    .order_by(SYMBOL_ID.asc())
    .fetch_all(&client)
    .await?;
  assert_eq!(rows.len(), 2, "the conflicting insert updated, not added");
  assert_eq!(rows.first().map(|row| row.indexed_at.as_str()), Some("t2"));
  Ok(())
}

#[tokio::test]
async fn fetch_optional_is_none_when_do_nothing_suppressed_the_write() -> TestResult {
  let client = db::setup_db("upsert_suppressed").await?;
  seed_memory(&client).await?;

  let suppressed: Option<MemoryId> = InsertBuilder::new("memories")
    .set(&MEMORY_ID, "m1")
    .set(&BODY, "incoming")
    .on_conflict(OnConflict::column(&MEMORY_ID).do_nothing())
    .returning(&MEMORY_ID)
    .fetch_optional(&client)
    .await?;
  assert!(suppressed.is_none(), "no row was written, so none returned");
  Ok(())
}
