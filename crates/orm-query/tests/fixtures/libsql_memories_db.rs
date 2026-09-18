//! In-memory libsql `memories` database seeded through `InsertBuilder`.

use toolu_orm_macros::FromRow;
use toolu_orm_query::insert::InsertBuilder;

use super::seed::{ACCESS_COUNT, BODY, CREATED_AT, ID, LAST_ACCESSED, SEED, SQLITE_DDL};

/// Field order matches `MEMORY_COLUMNS`, which is how the derive decodes.
#[derive(FromRow, Debug, Clone, PartialEq)]
pub struct Memory {
  pub id: String,
  pub body: String,
  pub created_at: String,
  pub last_accessed: Option<String>,
  pub access_count: i64,
}

/// Connects, creates the table and inserts the four seed rows.
///
/// # Errors
///
/// The underlying libsql or builder error.
pub async fn setup_db() -> Result<libsql::Connection, Box<dyn std::error::Error>> {
  let database = libsql::Builder::new_local(":memory:").build().await?;
  let conn = database.connect()?;
  conn.execute(SQLITE_DDL, ()).await?;
  for (id, body, created_at, last_accessed, access_count) in SEED {
    let builder = InsertBuilder::new("memories")
      .set(&ID, id)
      .set(&BODY, body)
      .set(&CREATED_AT, created_at)
      .set(&ACCESS_COUNT, access_count);
    let builder = match last_accessed {
      Some(stamp) => builder.set(&LAST_ACCESSED, stamp),
      None => builder.set_null(&LAST_ACCESSED),
    };
    builder.execute(&conn).await?;
  }
  Ok(conn)
}

/// The ids of `rows`, for order-sensitive assertions.
pub fn ids(rows: &[Memory]) -> Vec<&str> {
  rows.iter().map(|row| row.id.as_str()).collect()
}
