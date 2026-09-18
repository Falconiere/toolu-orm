//! AC-1 / AC-10: `LIKE … ESCAPE` with a bound escape character on Postgres.

use toolu_orm_core::expr::{like_pattern_literal, Scalar};
use toolu_orm_core::query_column::TextOps;

use crate::db::{self, ids, Memory};
use crate::seed::{BODY, CREATED_AT, CUTOFF, ID};
use crate::support::{all_memories, TestResult, ESCAPE};

#[tokio::test]
async fn like_escape_matches_only_the_literal_percent_row() -> TestResult {
  let client = db::setup_db("scalar_expr_wildcards").await?;
  let pattern = format!("%{}%", like_pattern_literal("100%", ESCAPE));

  let rows: Vec<Memory> = all_memories()
    .filter(BODY.like_escape(pattern, ESCAPE))
    .order_by(ID.asc())
    .fetch_all(&client)
    .await?;

  assert_eq!(ids(&rows), vec!["m1"]);
  Ok(())
}

#[tokio::test]
async fn the_same_search_without_an_escape_matches_both_percent_rows() -> TestResult {
  let client = db::setup_db("scalar_expr_wildcards_plain").await?;

  let rows: Vec<Memory> = all_memories()
    .filter(BODY.like("%100%%"))
    .order_by(ID.asc())
    .fetch_all(&client)
    .await?;

  assert_eq!(ids(&rows), vec!["m1", "m2"]);
  Ok(())
}

#[tokio::test]
async fn like_escape_matches_only_the_literal_underscore_row() -> TestResult {
  let client = db::setup_db("scalar_expr_wildcards_underscore").await?;

  let rows: Vec<Memory> = all_memories()
    .filter(BODY.like_escape(like_pattern_literal("a_b", ESCAPE), ESCAPE))
    .fetch_all(&client)
    .await?;

  assert_eq!(ids(&rows), vec!["m3"]);
  Ok(())
}

/// The SQLite suite compares the same cutoff through `datetime(...)`; this is
/// the dialect-neutral half of that comparison, text against text.
#[tokio::test]
async fn a_bound_scalar_comparison_selects_the_row_at_the_cutoff() -> TestResult {
  let client = db::setup_db("scalar_expr_cutoff").await?;

  let rows: Vec<Memory> = all_memories()
    .filter(Scalar::col(&CREATED_AT).gte(Scalar::bind(CUTOFF)))
    .order_by(ID.asc())
    .fetch_all(&client)
    .await?;

  assert_eq!(ids(&rows), vec!["m1"]);
  Ok(())
}
