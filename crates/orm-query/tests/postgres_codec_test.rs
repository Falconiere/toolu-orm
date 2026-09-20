//! Insert and filter one row through the Postgres column codecs.
//!
//! Needs `docker compose -f docker-compose.test.yaml up -d --wait` and
//! `TEST_DB_PORT=5434` locally. The timestamp is also read back with
//! `to_char`, so an epoch bind and an RFC3339 bind cannot pass by agreeing
//! with each other.

#[path = "fixtures/postgres_db.rs"]
pub mod pg;

use toolu_orm_core::column::{Boolean, Jsonb, Numeric, Text, Timestamp, Uuid};
use toolu_orm_core::query_column::{Column, CommonOps};
use toolu_orm_macros::FromRow;
use toolu_orm_query::insert::InsertBuilder;
use toolu_orm_query::select::SelectBuilder;

use pg::{client, TestResult};

const ID: Column<Text> = Column::new("typed", "id");
const ACTIVE: Column<Boolean> = Column::new("typed", "active");
const SEEN: Column<Timestamp> = Column::new("typed", "seen");
const META: Column<Jsonb> = Column::new("typed", "meta");
const TOKEN: Column<Uuid> = Column::new("typed", "token");
const AMOUNT: Column<Numeric> = Column::new("typed", "amount");

const ROW: &str = "row-1";
const TOKEN_TEXT: &str = "11111111-1111-1111-1111-111111111111";
const META_TEXT: &str = r#"{"ok":true}"#;
const AMOUNT_TEXT: &str = "12.50";
const EPOCH: &str = "1970-01-01T00:00:00Z";

#[derive(FromRow, Debug)]
struct IdRow {
  id: String,
}

#[tokio::test]
async fn insert_and_filter_native_column_types() -> TestResult {
  let client = client("q_pg_codecs").await?;
  client
    .batch_execute(
      "CREATE TABLE typed (
         id TEXT PRIMARY KEY,
         active BOOLEAN NOT NULL,
         seen TIMESTAMPTZ NOT NULL,
         meta JSONB NOT NULL,
         token UUID NOT NULL,
         amount NUMERIC NOT NULL
       )",
    )
    .await?;

  let inserted = InsertBuilder::new("typed")
    .set(&ID, ROW)
    .set(&ACTIVE, true)
    .set(&SEEN, 0_i64)
    .set(&META, META_TEXT)
    .set(&TOKEN, TOKEN_TEXT)
    .set(&AMOUNT, AMOUNT_TEXT)
    .execute(&client)
    .await?;
  assert_eq!(inserted, 1);

  assert_eq!(
    id_where(&client, ACTIVE.eq(true)).await?,
    Some(ROW.to_owned())
  );
  assert_eq!(
    id_where(&client, SEEN.eq(EPOCH)).await?,
    Some(ROW.to_owned())
  );
  assert_eq!(
    id_where(&client, META.eq(META_TEXT)).await?,
    Some(ROW.to_owned())
  );
  assert_eq!(
    id_where(&client, TOKEN.eq(TOKEN_TEXT)).await?,
    Some(ROW.to_owned())
  );
  assert_eq!(
    id_where(&client, AMOUNT.eq(AMOUNT_TEXT)).await?,
    Some(ROW.to_owned())
  );
  assert_eq!(id_where(&client, ACTIVE.eq(false)).await?, None);

  let seen: String = client
    .query_one(
      r#"SELECT to_char(seen AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"')
         FROM typed WHERE id = $1"#,
      &[&ROW],
    )
    .await?
    .try_get(0)?;
  assert_eq!(seen, EPOCH);
  Ok(())
}

async fn id_where(
  client: &tokio_postgres::Client,
  filter: toolu_orm_core::expr::Expr,
) -> Result<Option<String>, toolu_orm_query::QueryError> {
  let row: Option<IdRow> = SelectBuilder::new("typed")
    .columns_raw(&["id"])
    .filter(filter)
    .fetch_optional(client)
    .await?;
  Ok(row.map(|found| found.id))
}
