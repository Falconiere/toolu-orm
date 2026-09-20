//! Partial-index conflict targets and `DO UPDATE` guards.

use toolu_orm_core::expr::Scalar;
use toolu_orm_core::query_column::CommonOps;
use toolu_orm_query::insert::OnConflict;
use toolu_orm_query::update::UpdateBuilder;

use super::support::{
  insert_product, live_upsert, products, setup, sole_name, Product, ProductName, TestResult,
  DELETED, NAME, SKU,
};

#[test]
fn an_update_guard_skips_the_row_when_it_fails() -> TestResult {
  let conn = setup()?;
  insert_product(&conn, "pen", 0, "locked")?;

  let skipped: Option<ProductName> = live_upsert("renamed")
    .on_conflict(
      OnConflict::column(&SKU)
        .where_target(Scalar::col(&DELETED).eq(Scalar::sql("0")))
        .set_excluded(&NAME)
        .where_update(NAME.eq("open")),
    )
    .returning(&NAME)
    .fetch_optional(&conn)?;
  assert!(
    skipped.is_none(),
    "a false guard writes nothing and returns nothing"
  );
  assert_eq!(sole_name(products(&conn)?)?, "locked");

  UpdateBuilder::new("products")
    .set(&NAME, "open")
    .filter(SKU.eq("pen"))
    .execute(&conn)?;
  let written: ProductName = live_upsert("renamed")
    .on_conflict(
      OnConflict::column(&SKU)
        .where_target(Scalar::col(&DELETED).eq(Scalar::sql("0")))
        .set_excluded(&NAME)
        .where_update(NAME.eq("open")),
    )
    .returning(&NAME)
    .fetch_one(&conn)?;
  assert_eq!(
    written.name, "renamed",
    "the same guard writes when it holds"
  );
  Ok(())
}

#[test]
fn an_index_predicate_misses_a_row_the_partial_index_excludes() -> TestResult {
  let conn = setup()?;
  insert_product(&conn, "pen", 1, "gone")?;

  live_upsert("live").execute(&conn)?;
  assert_eq!(
    products(&conn)?,
    vec![
      Product {
        sku: "pen".into(),
        deleted: 0,
        name: "live".into()
      },
      Product {
        sku: "pen".into(),
        deleted: 1,
        name: "gone".into()
      },
    ],
    "the tombstone is not a conflict for the live partial index"
  );

  live_upsert("renamed").execute(&conn)?;
  assert_eq!(
    products(&conn)?,
    vec![
      Product {
        sku: "pen".into(),
        deleted: 0,
        name: "renamed".into()
      },
      Product {
        sku: "pen".into(),
        deleted: 1,
        name: "gone".into()
      },
    ],
    "the second live insert updates the live row and leaves the tombstone"
  );
  Ok(())
}
