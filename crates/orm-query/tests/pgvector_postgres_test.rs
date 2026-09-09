//! Live Postgres pgvector: L2 / cosine / neg-IP distance `ORDER BY … LIMIT k`.
//!
//! Requires `TEST_DB_PORT=5434` and a pgvector-capable server
//! (`pgvector/pgvector:pg16`). Fails hard when the server or extension is absent.

#[path = "fixtures/pgvector_schema.rs"]
pub mod schema;

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::OrderBy;
use toolu_orm_core::pgvector::PgVectorOps;
use toolu_orm_macros::FromRow;
use toolu_orm_query::select::SelectBuilder;

use schema::{EMBEDDING, SEED};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[derive(FromRow, Debug, Clone, PartialEq)]
struct Hit {
  id: String,
}

#[derive(FromRow, Debug, Clone, PartialEq)]
struct ScoredHit {
  id: String,
  distance: f64,
}

async fn seeded_client(
  schema_name: &str,
) -> Result<tokio_postgres::Client, Box<dyn std::error::Error>> {
  let conn_str = format!(
    "host={} port={} user={} password={} dbname={}",
    std::env::var("TEST_DB_HOST").unwrap_or_else(|_| "localhost".to_owned()),
    std::env::var("TEST_DB_PORT").unwrap_or_else(|_| "5434".to_owned()),
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
    .batch_execute("CREATE EXTENSION IF NOT EXISTS vector")
    .await
    .map_err(|e| {
      format!("pgvector extension unavailable (need image pgvector/pgvector:pg16): {e}")
    })?;

  client
    .batch_execute(&format!(
      "DROP SCHEMA IF EXISTS {schema_name} CASCADE; CREATE SCHEMA {schema_name}; \
       SET search_path TO {schema_name}, public; {}",
      schema::DDL
    ))
    .await?;

  for (id, embedding) in SEED {
    let embedding = *embedding;
    client
      .execute(
        "INSERT INTO items (id, embedding) VALUES ($1, CAST($2 AS text)::vector)",
        &[&id, &embedding],
      )
      .await?;
  }
  Ok(client)
}

#[tokio::test]
async fn l2_top_k_returns_nearest_neighbour_first() -> TestResult {
  let client = seeded_client("q_pgvec_l2").await?;
  let dist = EMBEDDING.l2_distance_for(Dialect::Postgres, &[1.0, 0.0, 0.0])?;

  let hits: Vec<ScoredHit> = SelectBuilder::new("items")
    .columns_raw(&["id"])
    .column_expr(&format!("({})::float8", dist.sql()), "distance")
    .order_by(OrderBy::alias_asc("distance"))
    .limit(2)
    .fetch_all(&client)
    .await?;

  assert_eq!(
    hits.iter().map(|h| h.id.as_str()).collect::<Vec<_>>(),
    vec!["near", "origin"]
  );
  let first = hits.first().ok_or("expected a hit")?;
  let second = hits.get(1).ok_or("expected a second hit")?;
  assert!(first.distance < second.distance);
  Ok(())
}

#[tokio::test]
async fn order_by_distance_without_projection_still_ranks() -> TestResult {
  let client = seeded_client("q_pgvec_order").await?;
  let dist = EMBEDDING.l2_distance_for(Dialect::Postgres, &[1.0, 0.0, 0.0])?;

  let hits: Vec<Hit> = SelectBuilder::new("items")
    .columns_raw(&["id"])
    .order_by(dist)
    .limit(1)
    .fetch_all(&client)
    .await?;

  assert_eq!(hits, vec![Hit { id: "near".into() }]);
  Ok(())
}

#[tokio::test]
async fn cosine_and_neg_inner_product_run_on_live_server() -> TestResult {
  let client = seeded_client("q_pgvec_ops").await?;

  let cosine = EMBEDDING.cosine_distance_for(Dialect::Postgres, &[1.0, 0.0, 0.0])?;
  let hits: Vec<Hit> = SelectBuilder::new("items")
    .columns_raw(&["id"])
    .order_by(cosine.asc())
    .limit(1)
    .fetch_all(&client)
    .await?;
  let nearest = hits.first().ok_or("expected a cosine hit")?;
  assert_eq!(nearest.id, "near");

  let nip = EMBEDDING.neg_inner_product_for(Dialect::Postgres, &[1.0, 0.0, 0.0])?;
  let hits: Vec<Hit> = SelectBuilder::new("items")
    .columns_raw(&["id"])
    .order_by(nip.asc())
    .limit(1)
    .fetch_all(&client)
    .await?;
  // Highest inner product with [1,0,0] among seeds is `far` ([10,0,0]);
  // `<#>` is negative IP so ASC picks the largest IP first.
  let nearest = hits.first().ok_or("expected a neg-IP hit")?;
  assert_eq!(nearest.id, "far");
  Ok(())
}
