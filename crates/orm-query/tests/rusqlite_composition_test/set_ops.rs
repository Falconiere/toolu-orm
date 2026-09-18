//! `UNION` and `UNION ALL` over two real lookup branches that overlap on one
//! row.
//!
//! The compounds order by the **output** alias, not by a qualified column: a
//! compound's result has no table qualification, and Postgres rejects
//! `ORDER BY "code_symbols"."id"` there with *missing FROM-clause entry* while
//! SQLite tolerates it. `OrderBy::alias_asc` is the portable form.

use crate::db::{ids, setup_db, IdRow};
use toolu_orm_core::expr::OrderBy;

use crate::queries::{by_path, by_repo};

type Outcome = Result<(), Box<dyn std::error::Error>>;

/// `r1` holds `c1, c2`; `src/a.rs` holds `c1, c3`. `UNION` collapses the
/// shared `c1`, so three rows — distinct from either arm's two and from the
/// four `UNION ALL` returns.
#[test]
fn union_collapses_the_row_both_branches_return() -> Outcome {
  let conn = setup_db()?;

  let rows: Vec<IdRow> = by_repo("r1")
    .union(by_path("src/a.rs"))
    .order_by(OrderBy::alias_asc("id"))
    .fetch_all(&conn)?;

  assert_eq!(ids(&rows), vec!["c1", "c2", "c3"]);
  Ok(())
}

#[test]
fn union_all_keeps_every_row_of_both_branches() -> Outcome {
  let conn = setup_db()?;

  let rows: Vec<IdRow> = by_repo("r1")
    .union_all(by_path("src/a.rs"))
    .order_by(OrderBy::alias_asc("id"))
    .fetch_all(&conn)?;

  assert_eq!(ids(&rows), vec!["c1", "c1", "c2", "c3"]);
  Ok(())
}

/// Three branches, chained and nested, must return the same rows — the
/// flattening proven on the rendered SQL, now proven on the data.
#[test]
fn a_nested_compound_returns_the_same_rows_as_a_chained_one() -> Outcome {
  let conn = setup_db()?;

  let chained: Vec<IdRow> = by_repo("r1")
    .union(by_path("src/a.rs"))
    .union(by_repo("r2"))
    .order_by(OrderBy::alias_asc("id"))
    .fetch_all(&conn)?;
  let nested: Vec<IdRow> = by_repo("r1")
    .union(by_path("src/a.rs").union(by_repo("r2")))
    .order_by(OrderBy::alias_asc("id"))
    .fetch_all(&conn)?;

  assert_eq!(ids(&chained), vec!["c1", "c2", "c3"]);
  assert_eq!(chained, nested);
  Ok(())
}

/// The tail bounds the merged result, not one arm: the first two of the three
/// distinct ids.
#[test]
fn the_row_window_applies_to_the_whole_compound() -> Outcome {
  let conn = setup_db()?;

  let rows: Vec<IdRow> = by_repo("r1")
    .union(by_path("src/a.rs"))
    .order_by(OrderBy::alias_asc("id"))
    .limit(2)
    .fetch_all(&conn)?;

  assert_eq!(ids(&rows), vec!["c1", "c2"]);
  Ok(())
}

/// `count()` over a compound is the size of the merged set — not the four raw
/// rows, and not either arm's two.
#[test]
fn counting_a_compound_reports_the_deduplicated_size() -> Outcome {
  let conn = setup_db()?;

  assert_eq!(by_repo("r1").union(by_path("src/a.rs")).count(&conn)?, 3);
  assert_eq!(
    by_repo("r1").union_all(by_path("src/a.rs")).count(&conn)?,
    4
  );
  assert_eq!(by_repo("r1").count(&conn)?, 2);
  Ok(())
}

#[test]
fn exists_over_a_compound_follows_whether_any_arm_matches() -> Outcome {
  let conn = setup_db()?;

  assert!(by_repo("r9").union(by_path("src/a.rs")).exists(&conn)?);
  assert!(!by_repo("r9").union(by_path("nope.rs")).exists(&conn)?);
  Ok(())
}
