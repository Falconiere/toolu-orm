//! Live Postgres FTS: `@@`, `ts_rank`, weights, stemming, websearch.
//!
//! Requires `TEST_DB_PORT=5434` (or a reachable server). Fails hard when absent.

#[path = "fixtures/pg_fts_schema.rs"]
pub mod schema;

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::OrderBy;
use toolu_orm_core::pg_fts;
use toolu_orm_core::query_column::CommonOps;
use toolu_orm_macros::FromRow;
use toolu_orm_query::select::SelectBuilder;

use schema::{DELETED_AT, ID, SEARCH_VECTOR, SEED};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[derive(FromRow, Debug, Clone, PartialEq)]
struct Hit {
  id: String,
}

#[derive(FromRow, Debug, Clone, PartialEq)]
struct ScoredHit {
  id: String,
  score: f64,
}

async fn seeded_client(
  schema_name: &str,
) -> Result<tokio_postgres::Client, Box<dyn std::error::Error>> {
  let conn_str = format!(
    "host={} port={} user={} password={} dbname={}",
    std::env::var("TEST_DB_HOST").unwrap_or_else(|_| "localhost".to_owned()),
    std::env::var("TEST_DB_PORT").unwrap_or_else(|_| "5433".to_owned()),
    std::env::var("TEST_DB_USER").unwrap_or_else(|_| "toolu".to_owned()),
    std::env::var("TEST_DB_PASSWORD").unwrap_or_else(|_| "toolu".to_owned()),
    std::env::var("TEST_DB_NAME").unwrap_or_else(|_| "toolu".to_owned()),
  );
  let (client, connection) = tokio_postgres::connect(&conn_str, tokio_postgres::NoTls).await?;
  tokio::spawn(async move {
    if let Err(e) = connection.await {
      eprintln!("postgres connection task ended with error: {e}");
    }
  });
  client
    .batch_execute(&format!(
      "DROP SCHEMA IF EXISTS {schema_name} CASCADE; CREATE SCHEMA {schema_name}; \
       SET search_path TO {schema_name}; {}",
      schema::DDL
    ))
    .await?;
  for (id, body, tags, deleted) in SEED {
    client
      .execute(
        "INSERT INTO docs (id, body, tags, deleted_at) VALUES ($1, $2, $3, $4)",
        &[&id, &body, &tags, deleted],
      )
      .await?;
  }
  Ok(client)
}

#[tokio::test]
async fn runner_ranks_m2_then_m3_and_excludes_soft_deleted() -> TestResult {
  let client = seeded_client("q_pgfts_rank").await?;
  let doc = pg_fts::column_for(Dialect::Postgres, &SEARCH_VECTOR)?;
  let score = pg_fts::ts_rank_tsquery_for(Dialect::Postgres, &doc, "english", "runner", None)?;

  let hits: Vec<ScoredHit> = SelectBuilder::new("docs")
    .columns_raw(&["id"])
    .column_expr(&format!("{}::float8", score.sql()), "score")
    .filter(doc.matches_tsquery_for(Dialect::Postgres, "english", "runner")?)
    .filter(DELETED_AT.is_null())
    .order_by(OrderBy::alias_desc("score"))
    .fetch_all(&client)
    .await?;

  assert_eq!(
    hits.iter().map(|h| h.id.as_str()).collect::<Vec<_>>(),
    vec!["m2", "m3"]
  );
  assert!(
    hits.iter().all(|h| h.score > 0.0),
    "ts_rank must be positive"
  );
  let first = hits.first().ok_or("expected at least one hit")?;
  let second = hits.get(1).ok_or("expected a second hit")?;
  assert!(first.score > second.score);
  Ok(())
}

#[tokio::test]
async fn column_weights_change_which_row_wins() -> TestResult {
  let client = seeded_client("q_pgfts_weights").await?;
  let doc = pg_fts::column_for(Dialect::Postgres, &SEARCH_VECTOR)?;

  let a_only = pg_fts::ts_rank_tsquery_for(
    Dialect::Postgres,
    &doc,
    "english",
    "runner",
    Some(&[0.0, 0.0, 0.0, 1.0]),
  )?;
  let a_hits: Vec<ScoredHit> = SelectBuilder::new("docs")
    .columns_raw(&["id"])
    .column_expr(&format!("{}::float8", a_only.sql()), "score")
    .filter(doc.matches_tsquery_for(Dialect::Postgres, "english", "runner")?)
    .order_by(OrderBy::alias_desc("score"))
    .fetch_all(&client)
    .await?;
  assert_eq!(
    a_hits.first().map(|h| h.id.as_str()),
    Some("m2"),
    "A-only weights should prefer the body hit"
  );
  assert!(a_hits
    .iter()
    .find(|h| h.id == "m3")
    .is_some_and(|h| h.score == 0.0));

  let b_only = pg_fts::ts_rank_tsquery_for(
    Dialect::Postgres,
    &doc,
    "english",
    "runner",
    Some(&[0.0, 0.0, 1.0, 0.0]),
  )?;
  let b_hits: Vec<ScoredHit> = SelectBuilder::new("docs")
    .columns_raw(&["id"])
    .column_expr(&format!("{}::float8", b_only.sql()), "score")
    .filter(doc.matches_tsquery_for(Dialect::Postgres, "english", "runner")?)
    .order_by(OrderBy::alias_desc("score"))
    .fetch_all(&client)
    .await?;
  assert_eq!(
    b_hits.first().map(|h| h.id.as_str()),
    Some("m3"),
    "B-only weights should prefer the tags hit"
  );
  assert!(b_hits
    .iter()
    .find(|h| h.id == "m2")
    .is_some_and(|h| h.score == 0.0));
  Ok(())
}

#[tokio::test]
async fn a_stemmed_term_matches_and_websearch_excludes() -> TestResult {
  let client = seeded_client("q_pgfts_stem").await?;
  let doc = pg_fts::column_for(Dialect::Postgres, &SEARCH_VECTOR)?;

  let stemmed: Vec<Hit> = SelectBuilder::new("docs")
    .columns_raw(&["id"])
    .filter(doc.matches_tsquery_for(Dialect::Postgres, "english", "run")?)
    .order_by(ID.asc())
    .fetch_all(&client)
    .await?;
  assert_eq!(
    stemmed.iter().map(|h| h.id.as_str()).collect::<Vec<_>>(),
    vec!["m1"]
  );

  let web: Vec<Hit> = SelectBuilder::new("docs")
    .columns_raw(&["id"])
    .filter(doc.matches_websearch_to_tsquery_for(Dialect::Postgres, "english", "runner -shoes")?)
    .order_by(ID.asc())
    .fetch_all(&client)
    .await?;
  assert_eq!(
    web.iter().map(|h| h.id.as_str()).collect::<Vec<_>>(),
    vec!["m2"]
  );
  Ok(())
}
