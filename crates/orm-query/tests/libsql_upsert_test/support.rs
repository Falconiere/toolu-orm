//! Seeding and reading helpers shared by this binary's scenario modules.

use toolu_orm_core::query_column::CommonOps;
use toolu_orm_query::insert::InsertBuilder;
use toolu_orm_query::select::SelectBuilder;

use super::schema::{
  Memory, MemoryLink, BODY, LAST_USED, LINK_COLUMNS, LINK_ID, LINK_MEMORY_ID, LINK_NOTE,
  MEMORY_COLUMNS, MEMORY_ID, USED_COUNT, WORKSPACE_ID,
};

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

/// `m1` as every scenario starts it: a body and workspace no upsert names,
/// a counter at 3, and a stale timestamp.
///
/// # Errors
///
/// The underlying driver or builder error.
pub async fn seed_memory(conn: &libsql::Connection) -> TestResult {
  InsertBuilder::new("memories")
    .set(&MEMORY_ID, "m1")
    .set(&BODY, "original")
    .set(&WORKSPACE_ID, "w1")
    .set(&USED_COUNT, 3_i64)
    .set(&LAST_USED, "t0")
    .execute(conn)
    .await?;
  Ok(())
}

/// A child row pointing at `memory_id` through `ON DELETE CASCADE`.
///
/// # Errors
///
/// The underlying driver or builder error.
pub async fn seed_link(conn: &libsql::Connection, id: &str, memory_id: &str) -> TestResult {
  InsertBuilder::new("memory_links")
    .set(&LINK_ID, id)
    .set(&LINK_MEMORY_ID, memory_id)
    .set(&LINK_NOTE, "keep me")
    .execute(conn)
    .await?;
  Ok(())
}

/// The one `memories` row with this id.
///
/// # Errors
///
/// The underlying driver, builder or row-mapping error.
pub async fn memory(
  conn: &libsql::Connection,
  id: &str,
) -> Result<Memory, toolu_orm_query::QueryError> {
  SelectBuilder::new("memories")
    .columns_raw(&MEMORY_COLUMNS)
    .filter(MEMORY_ID.eq(id))
    .fetch_one(conn)
    .await
}

/// Every `memory_links` row, so a cascade shows up as an empty vector.
///
/// # Errors
///
/// The underlying driver, builder or row-mapping error.
pub async fn links(
  conn: &libsql::Connection,
) -> Result<Vec<MemoryLink>, toolu_orm_query::QueryError> {
  SelectBuilder::new("memory_links")
    .columns_raw(&LINK_COLUMNS)
    .fetch_all(conn)
    .await
}
