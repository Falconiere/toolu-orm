//! Sharing across a statement boundary, executed: a CTE body, a `UNION` arm,
//! and a subquery each reached by one handle.

use toolu_orm_core::expr::{OrderBy, Scalar, SharedBind};
use toolu_orm_core::query_column::{CommonOps, SharedOps};
use toolu_orm_query::select::{Cte, SelectBuilder};

use crate::db::{setup_db, IdRow, WeightRow};
use crate::seed::{CANDIDATE, DST_ID, REL, SRC_ID, WEIGHT};

type Outcome = Result<(), Box<dyn std::error::Error>>;

#[test]
fn a_cte_body_and_the_outer_where_share_one_placeholder() -> Outcome {
  let conn = setup_db()?;
  let node = SharedBind::new(CANDIDATE);

  let body = SelectBuilder::new("edges")
    .column_as(&DST_ID, "id")
    .filter(SRC_ID.eq_shared(&node))
    .filter(REL.eq("co_changed"));

  let rows: Vec<IdRow> = SelectBuilder::new("edges")
    .column_as(&DST_ID, "id")
    .with(Cte::new("outgoing", body))
    .filter(SRC_ID.eq_shared(&node))
    .filter(Scalar::col(&DST_ID).in_subquery(SelectBuilder::new("outgoing").columns_raw(&["id"])))
    .order_by(OrderBy::alias_asc("id"))
    .fetch_all(&conn)?;

  // `file:r:7.rs` is reached twice (once as `co_changed`, once as `imports`)
  // and `file:r:outside.rs` once.
  assert_eq!(
    rows,
    vec![
      IdRow {
        id: "file:r:7.rs".to_owned()
      },
      IdRow {
        id: "file:r:7.rs".to_owned()
      },
      IdRow {
        id: "file:r:outside.rs".to_owned()
      }
    ]
  );
  Ok(())
}

#[test]
fn both_union_arms_read_the_same_bound_candidate() -> Outcome {
  let conn = setup_db()?;
  let node = SharedBind::new(CANDIDATE);

  let incoming = SelectBuilder::new("edges")
    .column_as(&SRC_ID, "id")
    .filter(DST_ID.eq_shared(&node))
    .filter(REL.eq("co_changed"));

  let rows: Vec<IdRow> = SelectBuilder::new("edges")
    .column_as(&DST_ID, "id")
    .filter(SRC_ID.eq_shared(&node))
    .filter(REL.eq("co_changed"))
    .union(incoming)
    .order_by(OrderBy::alias_asc("id"))
    .fetch_all(&conn)?;

  assert_eq!(
    rows,
    vec![
      IdRow {
        id: "file:r:11.rs".to_owned()
      },
      IdRow {
        id: "file:r:7.rs".to_owned()
      },
      IdRow {
        id: "file:r:outside.rs".to_owned()
      }
    ]
  );
  Ok(())
}

#[test]
fn a_scalar_subquery_and_the_outer_filter_share_one_placeholder() -> Outcome {
  let conn = setup_db()?;
  let node = SharedBind::new(CANDIDATE);

  let top = SelectBuilder::new("edges")
    .column_scalar(Scalar::max(Scalar::col(&WEIGHT)), "top")
    .filter(DST_ID.eq_shared(&node));

  let rows: Vec<WeightRow> = SelectBuilder::new("edges")
    .column_scalar(Scalar::subquery(top), "weight")
    .filter(SRC_ID.eq_shared(&node))
    .filter(REL.eq("imports"))
    .fetch_all(&conn)?;

  // The only edge *into* the candidate carries weight 11.
  assert_eq!(rows, vec![WeightRow { weight: 11 }]);
  Ok(())
}
