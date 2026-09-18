//! AC-1 on Postgres, which is stricter than SQLite: a `DISTINCT` query's
//! `ORDER BY` term must appear in the select list, so the projection is
//! qualified to match what `PATH.asc()` renders.

use crate::db::{self, paths, JustPath};
use crate::support::{distinct_paths, raw_paths, TestResult};

#[tokio::test]
async fn distinct_collapses_the_duplicate_paths() -> TestResult {
  let client = db::setup_db("grouping_distinct_a").await?;

  let rows: Vec<JustPath> = distinct_paths().fetch_all(&client).await?;

  assert_eq!(paths(&rows), vec!["src/a.rs", "src/b.rs", "src/c.rs"]);
  Ok(())
}

/// The same window over the same ordering returns different rows with and
/// without `DISTINCT`, so the deduplication is the engine's and it happens
/// before the page is cut.
#[tokio::test]
async fn a_page_of_a_distinct_listing_is_a_page_of_distinct_rows() -> TestResult {
  let client = db::setup_db("grouping_distinct_b").await?;

  let deduplicated: Vec<JustPath> = distinct_paths()
    .limit(2)
    .offset(1)
    .fetch_all(&client)
    .await?;
  let raw: Vec<JustPath> = raw_paths().limit(2).offset(1).fetch_all(&client).await?;

  assert_eq!(paths(&deduplicated), vec!["src/b.rs", "src/c.rs"]);
  assert_eq!(paths(&raw), vec!["src/a.rs", "src/b.rs"]);
  Ok(())
}
