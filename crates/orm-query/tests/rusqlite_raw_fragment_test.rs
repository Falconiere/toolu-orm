//! Issue #131 end-to-end on rusqlite: a raw fragment that writes its own `?N`
//! and is **not** the first thing in the statement must compare against the
//! value it bound, not against an earlier clause's.
//!
//! The libsql twin is `libsql_raw_fragment_test`, the Postgres one
//! `postgres_raw_fragment_test`; the rendering itself is pinned per dialect in
//! orm-core's `expr_raw_numbered_index_test`.

#[path = "fixtures/rusqlite_reusable_bind_db.rs"]
pub mod db;
#[path = "fixtures/reusable_bind_seed.rs"]
pub mod seed;

use toolu_orm_core::expr::Expr;
use toolu_orm_core::query_column::CommonOps;
use toolu_orm_core::value::Value;
use toolu_orm_query::select::SelectBuilder;

use db::{edge_triples, setup_db, EdgeRow};
use seed::{CANDIDATE, INCOMING, OUTGOING, REL, SRC_ID, WEIGHT};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn a_reused_number_in_a_later_fragment_reads_its_own_value() -> TestResult {
  let conn = setup_db()?;

  let rows: Vec<EdgeRow> = SelectBuilder::new("edges")
    .columns_raw(&["src_id", "dst_id", "weight"])
    .filter(REL.eq("co_changed"))
    .filter(Expr::raw(
      r#"("edges"."src_id" = ?1 OR "edges"."dst_id" = ?1)"#,
      vec![Value::from(CANDIDATE)],
    ))
    .order_by(WEIGHT.asc())
    .fetch_all(&conn)?;

  // Every `co_changed` edge touching the candidate, in either direction; the
  // `imports` row (weight 100) is excluded by the first filter. Before the fix
  // `?1` was the `rel` value, so this returned nothing at all.
  assert_eq!(
    edge_triples(&rows),
    vec![
      (CANDIDATE, OUTGOING.0, OUTGOING.1),
      (INCOMING.0, CANDIDATE, INCOMING.1),
      (CANDIDATE, "file:r:outside.rs", 50),
    ]
  );
  Ok(())
}

#[test]
fn a_bare_placeholder_after_a_numbered_one_takes_the_next_value() -> TestResult {
  let conn = setup_db()?;

  // `?1` names the first of the fragment's own values twice; the bare `?`
  // that follows takes the second, one past the largest number assigned.
  let rows: Vec<EdgeRow> = SelectBuilder::new("edges")
    .columns_raw(&["src_id", "dst_id", "weight"])
    .filter(REL.eq("co_changed"))
    .filter(Expr::raw(
      r#"("edges"."src_id" = ?1 OR "edges"."dst_id" = ?1) AND "edges"."weight" > ?"#,
      vec![Value::from(CANDIDATE), Value::Integer(10)],
    ))
    .order_by(WEIGHT.asc())
    .fetch_all(&conn)?;

  assert_eq!(
    edge_triples(&rows),
    vec![
      (INCOMING.0, CANDIDATE, INCOMING.1),
      (CANDIDATE, "file:r:outside.rs", 50),
    ]
  );
  Ok(())
}

#[test]
fn two_numbers_out_of_reading_order_bound_the_right_way_round() -> TestResult {
  let conn = setup_db()?;

  // `?2` is the low bound and `?1` the high one: numbering by position rather
  // than by the author's index would invert the range.
  let rows: Vec<EdgeRow> = SelectBuilder::new("edges")
    .columns_raw(&["src_id", "dst_id", "weight"])
    .filter(SRC_ID.eq(CANDIDATE))
    .filter(Expr::raw(
      r#""edges"."weight" BETWEEN ?2 AND ?1"#,
      vec![Value::Integer(60), Value::Integer(10)],
    ))
    .order_by(WEIGHT.asc())
    .fetch_all(&conn)?;

  assert_eq!(
    edge_triples(&rows),
    vec![(CANDIDATE, "file:r:outside.rs", 50)]
  );
  Ok(())
}
