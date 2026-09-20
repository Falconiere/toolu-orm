//! Shared products fixture for this binary.

use toolu_orm_core::column::{Integer, Text};
use toolu_orm_core::expr::Scalar;
use toolu_orm_core::query_column::Column;
use toolu_orm_macros::FromRow;
use toolu_orm_query::insert::{InsertBuilder, OnConflict};
use toolu_orm_query::select::SelectBuilder;
use toolu_orm_query::QueryError;

pub const SKU: Column<Text> = Column::new("products", "sku");
pub const DELETED: Column<Integer> = Column::new("products", "deleted");
pub const NAME: Column<Text> = Column::new("products", "name");

const COLUMNS: [&str; 3] = ["sku", "deleted", "name"];

const DDL: &str = "
  CREATE TABLE products (
    sku TEXT NOT NULL,
    deleted INTEGER NOT NULL,
    name TEXT NOT NULL
  );
  CREATE UNIQUE INDEX products_live_sku ON products (sku) WHERE deleted = 0;
";

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

#[derive(FromRow, Debug, Clone, PartialEq)]
pub struct Product {
  pub sku: String,
  pub deleted: i64,
  pub name: String,
}

#[derive(FromRow, Debug, Clone, PartialEq)]
pub struct ProductName {
  pub name: String,
}

pub fn setup() -> Result<rusqlite::Connection, rusqlite::Error> {
  let conn = rusqlite::Connection::open_in_memory()?;
  conn.execute_batch(DDL)?;
  Ok(conn)
}

pub fn insert_product(
  conn: &rusqlite::Connection,
  sku: &str,
  deleted: i64,
  name: &str,
) -> TestResult {
  InsertBuilder::new("products")
    .set(&SKU, sku)
    .set(&DELETED, deleted)
    .set(&NAME, name)
    .execute(conn)?;
  Ok(())
}

pub fn products(conn: &rusqlite::Connection) -> Result<Vec<Product>, QueryError> {
  SelectBuilder::new("products")
    .columns_raw(&COLUMNS)
    .order_by(DELETED.asc())
    .fetch_all(conn)
}

pub fn sole_name(mut rows: Vec<Product>) -> Result<String, Box<dyn std::error::Error>> {
  if rows.len() != 1 {
    return Err(format!("expected one product, found {}", rows.len()).into());
  }
  rows
    .pop()
    .map(|row| row.name)
    .ok_or_else(|| "expected one product".into())
}

pub fn live_upsert(name: &str) -> InsertBuilder {
  InsertBuilder::new("products")
    .set(&SKU, "pen")
    .set(&DELETED, 0_i64)
    .set(&NAME, name)
    .on_conflict(
      OnConflict::column(&SKU)
        .where_target(Scalar::col(&DELETED).eq(Scalar::sql("0")))
        .set_excluded(&NAME),
    )
}
