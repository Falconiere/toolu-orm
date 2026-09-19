//! Issue #131 end-to-end on libsql: a raw fragment that writes its own `?N`
//! and is **not** the first thing in the statement must compare against the
//! value it bound, not against an earlier clause's.
//!
//! The rendering rule is pinned per dialect in orm-core's
//! `expr_raw_numbered_index_test`; what this suite proves is the failure mode
//! that rendering caused — the statement prepared successfully and answered
//! with the wrong rows. Before the fix the first test below returns **zero**
//! rows: `?1` compared `src_id`/`dst_id` against `"co_changed"`.

#[path = "fixtures/libsql_reusable_bind_db.rs"]
pub mod db;
#[path = "fixtures/reusable_bind_seed.rs"]
pub mod seed;

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::Expr;
use toolu_orm_core::query_column::CommonOps;
use toolu_orm_core::value::Value;
use toolu_orm_query::select::SelectBuilder;

use db::{edge_triples, setup_db, EdgeRow};
use seed::{CANDIDATE, INCOMING, OUTGOING, REL, SRC_ID, WEIGHT};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// `SELECT … WHERE rel = <bound> AND (src_id = ?1 OR dst_id = ?1)`, with the
/// raw fragment second so its placeholder cannot be index 1.
fn candidate_edges() -> SelectBuilder {
  SelectBuilder::new("edges")
    .columns_raw(&["src_id", "dst_id", "weight"])
    .filter(REL.eq("co_changed"))
    .filter(Expr::raw(
      r#"("edges"."src_id" = ?1 OR "edges"."dst_id" = ?1)"#,
      vec![Value::from(CANDIDATE)],
    ))
    .order_by(WEIGHT.asc())
}

#[tokio::test]
async fn a_reused_number_in_a_later_fragment_reads_its_own_value() -> TestResult {
  let conn = setup_db().await?;
  let rows: Vec<EdgeRow> = candidate_edges().fetch_all(&conn).await?;

  // Every `co_changed` edge touching the candidate, in either direction: the
  // `imports` row (weight 100) is excluded by the first filter.
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
async fn the_statement_that_ran_names_the_fragments_own_index_twice() -> TestResult {
  let (sql, params) = candidate_edges().to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    concat!(
      r#"SELECT "src_id", "dst_id", "weight" FROM "edges""#,
      r#" WHERE "edges"."rel" = ?1"#,
      r#" AND ("edges"."src_id" = ?2 OR "edges"."dst_id" = ?2)"#,
      r#" ORDER BY "edges"."weight" ASC"#
    )
  );
  // One value for the fragment however many times it names ?1.
  assert_eq!(
    params,
    vec![Value::from("co_changed"), Value::from(CANDIDATE)]
  );
  Ok(())
}

#[tokio::test]
async fn two_numbers_out_of_reading_order_bound_the_right_way_round() -> TestResult {
  let conn = setup_db().await?;

  // `?2` is the low bound and `?1` the high one, so an implementation that
  // numbered by position rather than by the author's index would invert the
  // range and return nothing.
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
