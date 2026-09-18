//! AC-1: `%` and `_` typed by a user match literally under `LIKE … ESCAPE`.

use toolu_orm_core::expr::like_pattern_literal;
use toolu_orm_core::query_column::TextOps;

use crate::db::{self, ids, Memory};
use crate::seed::{BODY, ID};
use crate::support::{all_memories, TestResult, ESCAPE};

#[test]
fn like_escape_matches_only_the_literal_percent_row() -> TestResult {
  let conn = db::setup_db()?;
  let pattern = format!("%{}%", like_pattern_literal("100%", ESCAPE));

  let rows: Vec<Memory> = all_memories()
    .filter(BODY.like_escape(pattern, ESCAPE))
    .order_by(ID.asc())
    .fetch_all(&conn)?;

  assert_eq!(ids(&rows), vec!["m1"]);
  Ok(())
}

#[test]
fn the_same_search_without_an_escape_matches_both_percent_rows() -> TestResult {
  let conn = db::setup_db()?;

  let rows: Vec<Memory> = all_memories()
    .filter(BODY.like("%100%%"))
    .order_by(ID.asc())
    .fetch_all(&conn)?;

  assert_eq!(ids(&rows), vec!["m1", "m2"]);
  Ok(())
}

#[test]
fn like_escape_matches_only_the_literal_underscore_row() -> TestResult {
  let conn = db::setup_db()?;

  let escaped: Vec<Memory> = all_memories()
    .filter(BODY.like_escape(like_pattern_literal("a_b", ESCAPE), ESCAPE))
    .fetch_all(&conn)?;
  assert_eq!(ids(&escaped), vec!["m3"]);

  let unescaped: Vec<Memory> = all_memories()
    .filter(BODY.like("a_b"))
    .order_by(ID.asc())
    .fetch_all(&conn)?;
  assert_eq!(ids(&unescaped), vec!["m3", "m4"]);
  Ok(())
}
