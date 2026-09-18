//! Sharing across clauses, statement boundaries and the mutation builders,
//! executed on libsql.

use toolu_orm_core::expr::{OrderBy, Scalar, SharedBind, SharedBindList};
use toolu_orm_core::query_column::{CommonOps, SharedOps};
use toolu_orm_query::delete::DeleteBuilder;
use toolu_orm_query::select::{Cte, SelectBuilder};
use toolu_orm_query::update::UpdateBuilder;

use crate::db::{edge_triples, setup_db, EdgeRow, IdRow, WeightRow};
use crate::seed::{CANDIDATE, DST_ID, REL, SRC_ID, WEIGHT_SUM};

type Outcome = Result<(), Box<dyn std::error::Error>>;

async fn all_edges(conn: &libsql::Connection) -> Result<Vec<EdgeRow>, Box<dyn std::error::Error>> {
  let rows: Vec<EdgeRow> = SelectBuilder::new("edges")
    .columns_raw(&["src_id", "dst_id", "weight"])
    .order_by(SRC_ID.asc())
    .order_by(DST_ID.asc())
    .fetch_all(conn)
    .await?;
  Ok(rows)
}

#[tokio::test]
async fn one_handle_spans_a_projection_a_filter_and_an_order_by() -> Outcome {
  let conn = setup_db().await?;
  let node = SharedBind::new(CANDIDATE);

  let rows: Vec<WeightRow> = SelectBuilder::new("edges")
    .column_expr(WEIGHT_SUM, "weight")
    .filter(SRC_ID.eq_shared(&node))
    .filter(DST_ID.ne_shared(&node))
    .group_by(&REL)
    .having(Scalar::count_star().gt(Scalar::bind(0)))
    .order_by(OrderBy::alias_asc("weight"))
    .fetch_all(&conn)
    .await?;

  // `imports` contributes 100; `co_changed` contributes 7 + 50.
  assert_eq!(
    rows,
    vec![WeightRow { weight: 57 }, WeightRow { weight: 100 }]
  );
  Ok(())
}

#[tokio::test]
async fn a_cte_body_and_the_outer_where_share_one_placeholder() -> Outcome {
  let conn = setup_db().await?;
  let node = SharedBind::new(CANDIDATE);

  let body = SelectBuilder::new("edges")
    .column_as(&DST_ID, "id")
    .filter(SRC_ID.eq_shared(&node))
    .filter(REL.eq("co_changed"));

  let rows: Vec<IdRow> = SelectBuilder::new("edges")
    .column_as(&DST_ID, "id")
    .with(Cte::new("outgoing", body))
    .filter(SRC_ID.eq_shared(&node))
    .filter(REL.eq("imports"))
    .filter(Scalar::col(&DST_ID).in_subquery(SelectBuilder::new("outgoing").columns_raw(&["id"])))
    .order_by(OrderBy::alias_asc("id"))
    .fetch_all(&conn)
    .await?;

  assert_eq!(
    rows,
    vec![IdRow {
      id: "file:r:7.rs".to_owned()
    }]
  );
  Ok(())
}

#[tokio::test]
async fn both_union_arms_read_the_same_bound_candidate() -> Outcome {
  let conn = setup_db().await?;
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
    .fetch_all(&conn)
    .await?;

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

#[tokio::test]
async fn an_update_shares_a_handle_between_set_and_where() -> Outcome {
  let conn = setup_db().await?;
  let renamed = SharedBind::new("file:r:renamed.rs");

  let changed = UpdateBuilder::new("edges")
    .set_scalar(&SRC_ID, Scalar::shared(&renamed))
    .filter(SRC_ID.ne_shared(&renamed))
    .filter(REL.eq("imports"))
    .execute(&conn)
    .await?;

  assert_eq!(changed, 1);
  let rows = all_edges(&conn).await?;
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

#[tokio::test]
async fn a_delete_shares_a_handle_across_both_orientations() -> Outcome {
  let conn = setup_db().await?;
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
    .execute(&conn)
    .await?;

  assert_eq!(removed, 2);
  let rows = all_edges(&conn).await?;
  assert_eq!(
    edge_triples(&rows),
    vec![
      (CANDIDATE, "file:r:7.rs", 100),
      (CANDIDATE, "file:r:outside.rs", 50),
    ]
  );
  Ok(())
}
