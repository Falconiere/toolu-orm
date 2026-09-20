//! `RETURNING` on update and delete, including a rolled-back write.

use toolu_orm_core::query_column::CommonOps;
use toolu_orm_query::delete::DeleteBuilder;
use toolu_orm_query::update::UpdateBuilder;
use toolu_orm_query::QueryError;

use super::support::{
  insert_product, products, setup, sole_name, ProductName, TestResult, NAME, SKU,
};

#[test]
fn update_returning_hands_back_the_written_row() -> TestResult {
  let conn = setup()?;
  insert_product(&conn, "pen", 0, "live")?;

  let written: ProductName = UpdateBuilder::new("products")
    .set(&NAME, "rewritten")
    .filter(SKU.eq("pen"))
    .returning(&NAME)
    .fetch_one(&conn)?;
  assert_eq!(written.name, "rewritten");
  assert_eq!(sole_name(products(&conn)?)?, "rewritten");
  Ok(())
}

#[test]
fn delete_returning_hands_back_the_removed_row() -> TestResult {
  let conn = setup()?;
  insert_product(&conn, "pen", 0, "live")?;

  let removed: ProductName = DeleteBuilder::new("products")
    .filter(SKU.eq("pen"))
    .returning(&NAME)
    .fetch_one(&conn)?;
  assert_eq!(
    removed.name, "live",
    "DELETE RETURNING projects the removed row"
  );
  assert!(products(&conn)?.is_empty());
  Ok(())
}

#[test]
fn fetch_one_reports_not_found_when_nothing_matched() -> TestResult {
  let conn = setup()?;
  insert_product(&conn, "pen", 0, "live")?;

  let outcome = UpdateBuilder::new("products")
    .set(&NAME, "rewritten")
    .filter(SKU.eq("missing"))
    .returning(&NAME)
    .fetch_one::<ProductName>(&conn);
  let Err(QueryError::NotFound { table }) = outcome else {
    return Err("a filter that matches nothing must report NotFound".into());
  };
  assert_eq!(table, "products");
  assert_eq!(sole_name(products(&conn)?)?, "live");
  Ok(())
}

#[test]
fn execute_refuses_a_returning_update_on_rusqlite() -> TestResult {
  let conn = setup()?;
  insert_product(&conn, "pen", 0, "live")?;

  let outcome = UpdateBuilder::new("products")
    .set(&NAME, "rewritten")
    .filter(SKU.eq("pen"))
    .returning(&NAME)
    .execute(&conn);
  let Err(QueryError::Driver(error)) = outcome else {
    return Err("rusqlite refuses to execute a row-producing statement".into());
  };
  assert!(
    error.to_string().contains("Execute returned results"),
    "unexpected driver error: {error}"
  );
  Ok(())
}

#[test]
fn rollback_discards_the_returned_update() -> TestResult {
  let conn = setup()?;
  insert_product(&conn, "pen", 0, "original")?;
  conn.execute_batch("BEGIN")?;

  let written: ProductName = UpdateBuilder::new("products")
    .set(&NAME, "rewritten")
    .filter(SKU.eq("pen"))
    .returning(&NAME)
    .fetch_one(&conn)?;
  assert_eq!(
    written.name, "rewritten",
    "the statement sees its own write"
  );

  conn.execute_batch("ROLLBACK")?;
  assert_eq!(sole_name(products(&conn)?)?, "original");
  Ok(())
}
