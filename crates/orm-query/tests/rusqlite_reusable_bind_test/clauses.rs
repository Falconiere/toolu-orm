//! Sharing across clauses and through the mutation builders, executed.

use toolu_orm_core::expr::{Scalar, SharedBind, SharedBindList};
use toolu_orm_core::query_column::{CommonOps, SharedOps};
use toolu_orm_query::delete::DeleteBuilder;
use toolu_orm_query::insert::{InsertBuilder, OnConflict};
use toolu_orm_query::select::SelectBuilder;
use toolu_orm_query::update::UpdateBuilder;

use crate::db::{edge_triples, setup_db, touched_pairs, EdgeRow, IdRow, TouchedRow, WeightRow};
use crate::seed::{
  CANDIDATE, DST_ID, DST_KIND, REL, SRC_ID, SRC_KIND, TOUCHED_DDL, TOUCHED_ID, TOUCHED_NOTE,
  WEIGHT, WEIGHT_SUM,
};

type Outcome = Result<(), Box<dyn std::error::Error>>;

/// Every edge left in the table, ordered so the assertion is deterministic.
fn all_edges(conn: &rusqlite::Connection) -> Result<Vec<EdgeRow>, Box<dyn std::error::Error>> {
  let rows: Vec<EdgeRow> = SelectBuilder::new("edges")
    .columns_raw(&["src_id", "dst_id", "weight"])
    .order_by(SRC_ID.asc())
    .order_by(DST_ID.asc())
    .fetch_all(conn)?;
  Ok(rows)
}

#[test]
fn one_handle_spans_a_projection_and_two_filters() -> Outcome {
  let conn = setup_db()?;
  let node = SharedBind::new(CANDIDATE);

  let rows: Vec<IdRow> = SelectBuilder::new("edges")
    .column_scalar(Scalar::shared(&node), "id")
    .filter(SRC_ID.eq_shared(&node))
    .filter(REL.eq("co_changed"))
    .filter(DST_ID.ne_shared(&node))
    .fetch_all(&conn)?;

  // Two `co_changed` rows leave the candidate: one into the working set, one
  // to the path outside it. Both project the handle's own value.
  assert_eq!(
    rows,
    vec![
      IdRow {
        id: CANDIDATE.to_owned()
      },
      IdRow {
        id: CANDIDATE.to_owned()
      }
    ]
  );
  Ok(())
}

#[test]
fn negated_shared_predicates_return_the_complementary_rows() -> Outcome {
  let conn = setup_db()?;
  let node = SharedBind::new(CANDIDATE);
  let inside = SharedBindList::new(["file:r:7.rs"]);

  let matched: Vec<WeightRow> = SelectBuilder::new("edges")
    .column_expr(WEIGHT_SUM, "weight")
    .filter(SRC_ID.eq_shared(&node))
    .filter(DST_ID.in_shared(&inside))
    .fetch_all(&conn)?;
  let rest: Vec<WeightRow> = SelectBuilder::new("edges")
    .column_expr(WEIGHT_SUM, "weight")
    .filter(SRC_ID.eq_shared(&node))
    .filter(DST_ID.not_in_shared(&inside))
    .fetch_all(&conn)?;

  // `candidate → file:r:7.rs` carries 7 as `co_changed` and 100 as `imports`.
  assert_eq!(matched, vec![WeightRow { weight: 107 }]);
  // Everything else leaving the candidate is the 50-weight outside edge.
  assert_eq!(rest, vec![WeightRow { weight: 50 }]);
  Ok(())
}

#[test]
fn a_shared_handle_counts_through_the_derived_table_wrap() -> Outcome {
  let conn = setup_db()?;
  let node = SharedBind::new(CANDIDATE);

  let groups = SelectBuilder::new("edges")
    .columns_qualified(&[&REL])
    .group_by(&REL)
    .filter(SRC_ID.eq_shared(&node))
    .filter(DST_ID.ne_shared(&node))
    .count(&conn)?;

  // Two relations leave the candidate: `co_changed` and `imports`.
  assert_eq!(groups, 2);
  Ok(())
}

#[test]
fn an_update_shares_a_handle_between_set_and_where() -> Outcome {
  let conn = setup_db()?;
  let renamed = SharedBind::new("file:r:renamed.rs");

  let changed = UpdateBuilder::new("edges")
    .set_scalar(&SRC_ID, Scalar::shared(&renamed))
    .filter(SRC_ID.ne_shared(&renamed))
    .filter(REL.eq("imports"))
    .execute(&conn)?;

  assert_eq!(changed, 1);
  let rows = all_edges(&conn)?;
  assert_eq!(
    edge_triples(&rows),
    vec![
      ("file:r:11.rs", CANDIDATE, 11),
      (CANDIDATE, "file:r:7.rs", 7),
      (CANDIDATE, "file:r:outside.rs", 50),
      ("file:r:renamed.rs", "file:r:7.rs", 100),
    ]
  );
  Ok(())
}

#[test]
fn a_delete_shares_a_handle_across_both_orientations() -> Outcome {
  let conn = setup_db()?;
  let node = SharedBind::new(CANDIDATE);
  let files = SharedBindList::new(["file:r:7.rs", "file:r:11.rs"]);

  let removed = DeleteBuilder::new("edges")
    .filter(REL.eq("co_changed"))
    .filter(
      SRC_ID
        .eq_shared(&node)
        .and(DST_ID.in_shared(&files))
        .or(DST_ID.eq_shared(&node).and(SRC_ID.in_shared(&files))),
    )
    .execute(&conn)?;

  // Both co-change edges go; the `imports` row and the outside-the-set row
  // stay, which is what the shared list not matching them proves.
  assert_eq!(removed, 2);
  let rows = all_edges(&conn)?;
  assert_eq!(
    edge_triples(&rows),
    vec![
      (CANDIDATE, "file:r:7.rs", 100),
      (CANDIDATE, "file:r:outside.rs", 50),
    ]
  );
  Ok(())
}

#[test]
fn an_insert_values_row_shares_a_handle_between_two_columns() -> Outcome {
  let conn = setup_db()?;
  let node = SharedBind::new("file:r:self.rs");

  InsertBuilder::new("edges")
    .set(&REL, "co_changed")
    .set(&SRC_KIND, "file")
    .set(&DST_KIND, "file")
    .set_scalar(&SRC_ID, Scalar::shared(&node))
    .set_scalar(&DST_ID, Scalar::shared(&node))
    .set(&WEIGHT, 3)
    .execute(&conn)?;

  let rows: Vec<EdgeRow> = SelectBuilder::new("edges")
    .columns_raw(&["src_id", "dst_id", "weight"])
    .filter(SRC_ID.eq_shared(&node))
    .filter(DST_ID.eq_shared(&node))
    .fetch_all(&conn)?;

  assert_eq!(
    edge_triples(&rows),
    vec![("file:r:self.rs", "file:r:self.rs", 3)]
  );
  Ok(())
}

#[test]
fn an_insert_select_shares_a_handle_with_its_conflict_assignment() -> Outcome {
  let conn = setup_db()?;
  conn.execute(TOUCHED_DDL, ())?;
  let node = SharedBind::new(CANDIDATE);
  // A row the source will collide with, so `DO UPDATE` actually fires.
  InsertBuilder::new("touched")
    .set(&TOUCHED_ID, "file:r:7.rs")
    .set(&TOUCHED_NOTE, "old")
    .execute(&conn)?;

  let source = SelectBuilder::new("edges")
    .column_as(&DST_ID, "id")
    .column_scalar(Scalar::sql("'seen'"), "note")
    .filter(SRC_ID.eq_shared(&node))
    .filter(REL.eq("co_changed"));

  InsertBuilder::new("touched")
    .select(&[&TOUCHED_ID, &TOUCHED_NOTE], source)
    .on_conflict(OnConflict::column(&TOUCHED_ID).set_scalar(&TOUCHED_NOTE, Scalar::shared(&node)))
    .execute(&conn)?;

  let rows: Vec<TouchedRow> = SelectBuilder::new("touched")
    .columns_raw(&["id", "note"])
    .order_by(TOUCHED_ID.asc())
    .fetch_all(&conn)?;

  // The colliding row took the *shared* handle's value, and the new row took
  // the source's literal — one placeholder served the filter and the update.
  assert_eq!(
    touched_pairs(&rows),
    vec![("file:r:7.rs", CANDIDATE), ("file:r:outside.rs", "seen"),]
  );
  Ok(())
}
