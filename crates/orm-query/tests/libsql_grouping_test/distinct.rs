//! AC-1: `DISTINCT` is applied by the engine before the page is cut.

use crate::db::{self, paths, JustPath};
use crate::support::{distinct_paths, raw_paths, TestResult};

/// `s1` has six rows over three distinct paths, so the deduplicated listing is
/// shorter than the raw one.
#[tokio::test]
async fn distinct_collapses_the_duplicate_paths() -> TestResult {
  let conn = db::setup_db().await?;

  let rows: Vec<JustPath> = distinct_paths().fetch_all(&conn).await?;

  assert_eq!(paths(&rows), vec!["src/a.rs", "src/b.rs", "src/c.rs"]);
  Ok(())
}

/// The discriminating case: the same window over the same ordering returns
/// different rows with and without `DISTINCT`. If the builder deduplicated a
/// page *after* fetching it, both would start at `src/a.rs`.
#[tokio::test]
async fn a_page_of_a_distinct_listing_is_a_page_of_distinct_rows() -> TestResult {
  let conn = db::setup_db().await?;

  let deduplicated: Vec<JustPath> = distinct_paths().limit(2).offset(1).fetch_all(&conn).await?;
  let raw: Vec<JustPath> = raw_paths().limit(2).offset(1).fetch_all(&conn).await?;

  assert_eq!(paths(&deduplicated), vec!["src/b.rs", "src/c.rs"]);
  assert_eq!(paths(&raw), vec!["src/a.rs", "src/b.rs"]);
  Ok(())
}

/// The full raw listing, so the duplicates the previous test relies on are
/// themselves asserted rather than assumed.
#[tokio::test]
async fn the_undeduplicated_listing_still_holds_every_duplicate() -> TestResult {
  let conn = db::setup_db().await?;

  let rows: Vec<JustPath> = raw_paths().fetch_all(&conn).await?;

  assert_eq!(
    paths(&rows),
    vec!["src/a.rs", "src/a.rs", "src/b.rs", "src/b.rs", "src/c.rs", "src/c.rs"]
  );
  Ok(())
}

/// `LIMIT` still bounds a distinct listing, and an offset past the end yields
/// nothing rather than failing.
#[tokio::test]
async fn an_offset_past_the_last_distinct_row_returns_no_rows() -> TestResult {
  let conn = db::setup_db().await?;

  let rows: Vec<JustPath> = distinct_paths().limit(5).offset(3).fetch_all(&conn).await?;

  assert!(rows.is_empty());
  Ok(())
}
