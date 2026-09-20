//! The same mutation clauses as `rusqlite_mutation_parity_test`, on live
//! Postgres. Needs `docker compose -f docker-compose.test.yaml up -d --wait`.

#[path = "fixtures/postgres_db.rs"]
pub mod pg;

use toolu_orm_core::column::{Integer, Text};
use toolu_orm_core::expr::Scalar;
use toolu_orm_core::query_column::{Column, CommonOps};
use toolu_orm_macros::FromRow;
use toolu_orm_query::delete::DeleteBuilder;
use toolu_orm_query::executor::PgTransaction;
use toolu_orm_query::insert::{InsertBuilder, OnConflict};
use toolu_orm_query::select::SelectBuilder;
use toolu_orm_query::update::UpdateBuilder;
use toolu_orm_query::QueryError;

use pg::TestResult;

const SKU: Column<Text> = Column::new("products", "sku");
const DELETED: Column<Integer> = Column::new("products", "deleted");
const NAME: Column<Text> = Column::new("products", "name");
const COLUMNS: [&str; 3] = ["sku", "deleted", "name"];

const DDL: &str = "
  CREATE TABLE products (
    sku TEXT NOT NULL,
    deleted BIGINT NOT NULL,
    name TEXT NOT NULL
  );
  CREATE UNIQUE INDEX products_live_sku ON products (sku) WHERE deleted = 0;
";

#[derive(FromRow, Debug, Clone, PartialEq)]
struct Product {
  sku: String,
  deleted: i64,
  name: String,
}

#[derive(FromRow, Debug, Clone, PartialEq)]
struct ProductName {
  name: String,
}

async fn products(
  exec: &(impl toolu_orm_query::executor::Executor + Send + Sync),
) -> Result<Vec<Product>, QueryError> {
  SelectBuilder::new("products")
    .columns_raw(&COLUMNS)
    .order_by(DELETED.asc())
    .fetch_all(exec)
    .await
}

async fn insert_product(
  exec: &(impl toolu_orm_query::executor::Executor + Send + Sync),
  sku: &str,
  deleted: i64,
  name: &str,
) -> TestResult {
  InsertBuilder::new("products")
    .set(&SKU, sku)
    .set(&DELETED, deleted)
    .set(&NAME, name)
    .execute(exec)
    .await?;
  Ok(())
}

fn sole_name(mut rows: Vec<Product>) -> Result<String, Box<dyn std::error::Error>> {
  if rows.len() != 1 {
    return Err(format!("expected one product, found {}", rows.len()).into());
  }
  rows
    .pop()
    .map(|row| row.name)
    .ok_or_else(|| "expected one product".into())
}

fn guarded(name: &str) -> InsertBuilder {
  InsertBuilder::new("products")
    .set(&SKU, "pen")
    .set(&DELETED, 0_i64)
    .set(&NAME, name)
    .on_conflict(
      OnConflict::column(&SKU)
        .where_target(Scalar::col(&DELETED).eq(Scalar::sql("0")))
        .set_excluded(&NAME)
        .where_update(NAME.eq("open")),
    )
}

fn live_upsert(name: &str) -> InsertBuilder {
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

#[tokio::test]
async fn update_returning_hands_back_the_written_row() -> TestResult {
  let client = pg::client("q_mut_update").await?;
  client.batch_execute(DDL).await?;
  insert_product(&client, "pen", 0, "live").await?;

  let written: ProductName = UpdateBuilder::new("products")
    .set(&NAME, "rewritten")
    .filter(SKU.eq("pen"))
    .returning(&NAME)
    .fetch_one(&client)
    .await?;
  assert_eq!(written.name, "rewritten");
  assert_eq!(sole_name(products(&client).await?)?, "rewritten");
  Ok(())
}

#[tokio::test]
async fn delete_returning_hands_back_the_removed_row() -> TestResult {
  let client = pg::client("q_mut_delete").await?;
  client.batch_execute(DDL).await?;
  insert_product(&client, "pen", 0, "live").await?;

  let removed: ProductName = DeleteBuilder::new("products")
    .filter(SKU.eq("pen"))
    .returning(&NAME)
    .fetch_one(&client)
    .await?;
  assert_eq!(removed.name, "live");
  assert!(products(&client).await?.is_empty());
  Ok(())
}

#[tokio::test]
async fn fetch_one_reports_not_found_when_nothing_matched() -> TestResult {
  let client = pg::client("q_mut_missing").await?;
  client.batch_execute(DDL).await?;
  insert_product(&client, "pen", 0, "live").await?;

  let outcome = UpdateBuilder::new("products")
    .set(&NAME, "rewritten")
    .filter(SKU.eq("missing"))
    .returning(&NAME)
    .fetch_one::<ProductName>(&client)
    .await;
  let Err(QueryError::NotFound { table }) = outcome else {
    return Err("a filter that matches nothing must report NotFound".into());
  };
  assert_eq!(table, "products");
  assert_eq!(sole_name(products(&client).await?)?, "live");
  Ok(())
}

#[tokio::test]
async fn an_update_guard_skips_the_row_when_it_fails() -> TestResult {
  let client = pg::client("q_mut_guard").await?;
  client.batch_execute(DDL).await?;
  insert_product(&client, "pen", 0, "locked").await?;

  let skipped: Option<ProductName> = guarded("renamed")
    .returning(&NAME)
    .fetch_optional(&client)
    .await?;
  assert!(skipped.is_none());
  assert_eq!(sole_name(products(&client).await?)?, "locked");

  UpdateBuilder::new("products")
    .set(&NAME, "open")
    .filter(SKU.eq("pen"))
    .execute(&client)
    .await?;
  let written: ProductName = guarded("renamed")
    .returning(&NAME)
    .fetch_one(&client)
    .await?;
  assert_eq!(written.name, "renamed");
  Ok(())
}

#[tokio::test]
async fn an_index_predicate_misses_a_row_the_partial_index_excludes() -> TestResult {
  let client = pg::client("q_mut_index").await?;
  client.batch_execute(DDL).await?;
  insert_product(&client, "pen", 1, "gone").await?;

  live_upsert("live").execute(&client).await?;
  live_upsert("renamed").execute(&client).await?;
  assert_eq!(
    products(&client).await?,
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
    ]
  );
  Ok(())
}

#[tokio::test]
async fn rollback_discards_the_returned_update() -> TestResult {
  let mut client = pg::client("q_mut_rollback").await?;
  client.batch_execute(DDL).await?;
  insert_product(&client, "pen", 0, "original").await?;

  let tx = PgTransaction::new(client.transaction().await?);
  let written: ProductName = UpdateBuilder::new("products")
    .set(&NAME, "rewritten")
    .filter(SKU.eq("pen"))
    .returning(&NAME)
    .fetch_one(&tx)
    .await?;
  assert_eq!(written.name, "rewritten");
  tx.rollback().await?;
  assert_eq!(sole_name(products(&client).await?)?, "original");
  Ok(())
}
