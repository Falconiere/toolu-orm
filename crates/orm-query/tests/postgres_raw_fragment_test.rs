//! Issue #131 end-to-end on a live Postgres (postgres lane, async), one schema
//! per test: a raw fragment that writes its own `?N` and is **not** first must
//! compare against the value it bound.
//!
//! Needs `docker compose -f docker-compose.test.yaml up -d --wait` and
//! `TEST_DB_PORT=5434` locally; an absent server fails the suite.
//!
//! Postgres is where the old rendering was loud rather than merely wrong: the
//! statement kept `$1` while the fragment's value landed at `$2`, so the server
//! rejected the bind ("bind message supplies 2 parameters, but prepared
//! statement requires 1"). SQLite quietly compared the wrong value instead.

#[path = "fixtures/pg_reusable_bind_db.rs"]
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

#[tokio::test]
async fn a_reused_number_in_a_later_fragment_reads_its_own_value() -> TestResult {
  let conn = setup_db("raw_fragment_reuse").await?;

  let rows: Vec<EdgeRow> = SelectBuilder::new("edges")
    .columns_raw(&["src_id", "dst_id", "weight"])
    .filter(REL.eq("co_changed"))
    .filter(Expr::raw(
      r#"("edges"."src_id" = ?1 OR "edges"."dst_id" = ?1)"#,
      vec![Value::from(CANDIDATE)],
    ))
    .order_by(WEIGHT.asc())
    .fetch_all(&conn)
    .await?;

  // One value for the fragment however many times it names `?1`, and it is the
  // candidate — not the `rel` value the old rendering pointed `$1` at.
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

#[tokio::test]
async fn a_bare_placeholder_after_a_numbered_one_takes_the_next_value() -> TestResult {
  let conn = setup_db("raw_fragment_bare_mix").await?;

  let rows: Vec<EdgeRow> = SelectBuilder::new("edges")
    .columns_raw(&["src_id", "dst_id", "weight"])
    .filter(REL.eq("co_changed"))
    .filter(Expr::raw(
      r#"("edges"."src_id" = ?1 OR "edges"."dst_id" = ?1) AND "edges"."weight" > ?"#,
      vec![Value::from(CANDIDATE), Value::Integer(10)],
    ))
    .order_by(WEIGHT.asc())
    .fetch_all(&conn)
    .await?;

  assert_eq!(
    edge_triples(&rows),
    vec![
      (INCOMING.0, CANDIDATE, INCOMING.1),
      (CANDIDATE, "file:r:outside.rs", 50),
    ]
  );
  Ok(())
}

#[tokio::test]
async fn two_numbers_out_of_reading_order_bound_the_right_way_round() -> TestResult {
  let conn = setup_db("raw_fragment_reverse").await?;

  let rows: Vec<EdgeRow> = SelectBuilder::new("edges")
    .columns_raw(&["src_id", "dst_id", "weight"])
    .filter(SRC_ID.eq(CANDIDATE))
    .filter(Expr::raw(
      r#""edges"."weight" BETWEEN ?2 AND ?1"#,
      vec![Value::Integer(60), Value::Integer(10)],
    ))
    .order_by(WEIGHT.asc())
    .fetch_all(&conn)
    .await?;

  assert_eq!(
    edge_triples(&rows),
    vec![(CANDIDATE, "file:r:outside.rs", 50)]
  );
  Ok(())
}
