//! `fetch_one` / `fetch_optional` decode at most one row on a live Postgres,
//! against a table holding 10,000 matching rows.
//!
//! The Postgres twin of `rusqlite_first_row_test`; see issue #87. Needs
//! `docker compose -f docker-compose.test.yaml up -d --wait` and
//! `TEST_DB_PORT=5434` locally; each test owns a schema.

#[path = "fixtures/counting_items.rs"]
pub mod items;
#[path = "fixtures/postgres_db.rs"]
pub mod pg;

use items::{
  decoded, reset, seed_sql_postgres, CountedId, FirstIdOnly, ITEM_DDL_POSTGRES, ROW_COUNT,
};
use pg::TestResult;
use toolu_orm_core::column::Integer;
use toolu_orm_core::query_column::{Column, NumericOps};
use toolu_orm_query::select::SelectBuilder;
use toolu_orm_query::QueryError;

const ID: Column<Integer> = Column::new("item", "id");

/// A client on its own schema whose `item` table holds ids `1..=rows`.
///
/// The seed's own row count is asserted, so every decode assertion below rests
/// on a table proven to hold `rows` rows rather than on an assumption.
async fn seeded(
  schema: &str,
  rows: i64,
) -> Result<tokio_postgres::Client, Box<dyn std::error::Error>> {
  let client = pg::client(schema).await?;
  client.batch_execute(ITEM_DDL_POSTGRES).await?;
  reseed(&client, rows).await?;
  Ok(client)
}

/// Replaces `item`'s contents with ids `1..=rows`, asserting the insert count.
async fn reseed(
  client: &tokio_postgres::Client,
  rows: i64,
) -> Result<(), Box<dyn std::error::Error>> {
  client.execute("TRUNCATE item", &[]).await?;
  let inserted = client
    .execute(seed_sql_postgres(rows).as_str(), &[])
    .await?;
  assert_eq!(
    i64::try_from(inserted)?,
    rows,
    "seed inserted the wrong count"
  );
  reset();
  Ok(())
}

fn items() -> SelectBuilder {
  SelectBuilder::new("item").columns_raw(&["id"])
}

#[tokio::test]
async fn fetch_one_decodes_one_row_for_a_large_match() -> TestResult {
  let client = seeded("q_pg_first_row_one", ROW_COUNT).await?;

  let first: CountedId = items().order_by(ID.asc()).fetch_one(&client).await?;

  assert_eq!(first.id, 1);
  assert_eq!(decoded(), 1, "fetch_one must decode exactly one row");
  Ok(())
}

#[tokio::test]
async fn fetch_optional_decodes_one_row_for_a_large_match() -> TestResult {
  let client = seeded("q_pg_first_row_optional", ROW_COUNT).await?;

  let first: Option<CountedId> = items().order_by(ID.asc()).fetch_optional(&client).await?;

  assert_eq!(first.map(|row| row.id), Some(1));
  assert_eq!(decoded(), 1, "fetch_optional must decode exactly one row");
  Ok(())
}

#[tokio::test]
async fn first_row_survives_a_later_row_that_cannot_decode() -> TestResult {
  let client = seeded("q_pg_first_row_bad_later", ROW_COUNT).await?;

  let one: FirstIdOnly = items().order_by(ID.asc()).fetch_one(&client).await?;
  let optional: Option<FirstIdOnly> = items().order_by(ID.asc()).fetch_optional(&client).await?;

  assert_eq!(one.id, 1);
  assert_eq!(optional.map(|row| row.id), Some(1));
  Ok(())
}

#[tokio::test]
async fn no_matching_row_is_not_found_and_none() -> TestResult {
  let client = seeded("q_pg_first_row_missing", ROW_COUNT).await?;

  let missing: Result<CountedId, QueryError> =
    items().filter(ID.gt(ROW_COUNT)).fetch_one(&client).await;
  let optional: Option<CountedId> = items()
    .filter(ID.gt(ROW_COUNT))
    .fetch_optional(&client)
    .await?;

  match missing {
    Err(QueryError::NotFound { table }) => assert_eq!(table, "item"),
    other => return Err(format!("expected NotFound, got: {other:?}").into()),
  }
  assert!(optional.is_none());
  assert_eq!(decoded(), 0);
  Ok(())
}

#[tokio::test]
async fn explicit_limit_zero_yields_no_row() -> TestResult {
  let client = seeded("q_pg_first_row_limit_zero", ROW_COUNT).await?;

  let one: Result<CountedId, QueryError> = items().limit(0).fetch_one(&client).await;
  let optional: Option<CountedId> = items().limit(0).fetch_optional(&client).await?;

  match one {
    Err(QueryError::NotFound { table }) => assert_eq!(table, "item"),
    other => return Err(format!("expected NotFound, got: {other:?}").into()),
  }
  assert!(optional.is_none());
  assert_eq!(decoded(), 0);
  Ok(())
}

#[tokio::test]
async fn filters_order_offset_and_positive_limit_still_select_the_first_row() -> TestResult {
  let client = seeded("q_pg_first_row_paging", ROW_COUNT).await?;

  let skipped: CountedId = items()
    .filter(ID.lte(ROW_COUNT))
    .order_by(ID.desc())
    .offset(2)
    .fetch_one(&client)
    .await?;
  assert_eq!(skipped.id, ROW_COUNT - 2);
  assert_eq!(decoded(), 1);

  reset();
  let paged: CountedId = items()
    .order_by(ID.asc())
    .limit(5)
    .fetch_one(&client)
    .await?;
  assert_eq!(paged.id, 1);
  assert_eq!(decoded(), 1, "an explicit page of 5 still decodes one row");
  Ok(())
}

#[tokio::test]
async fn decoded_rows_stay_one_as_cardinality_grows() -> TestResult {
  let client = seeded("q_pg_first_row_growth", 1).await?;

  for rows in [1, 10, 1_000, ROW_COUNT] {
    reseed(&client, rows).await?;

    let first: CountedId = items().order_by(ID.asc()).fetch_one(&client).await?;

    assert_eq!(first.id, 1);
    assert_eq!(decoded(), 1, "decoded more than one row at {rows} rows");
  }
  Ok(())
}
