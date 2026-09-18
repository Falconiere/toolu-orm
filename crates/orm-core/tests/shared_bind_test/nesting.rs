//! Reuse across nested `AND` / `OR`, on both dialects, with the rendered
//! index set checked programmatically rather than only by eye.

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::{Expr, SharedBind, SharedBindList};
use toolu_orm_core::query_column::{CommonOps, SharedOps};
use toolu_orm_core::value::Value;

use crate::fixtures::{sqlite_indices, text, DST_ID, REL, SRC_ID};

/// The issue's two-orientation predicate, in miniature.
fn both_directions(node: &SharedBind, files: &SharedBindList) -> Expr {
  SRC_ID
    .eq_shared(node)
    .and(DST_ID.in_shared(files))
    .or(DST_ID.eq_shared(node).and(SRC_ID.in_shared(files)))
}

#[test]
fn a_handle_reused_across_nested_and_or_takes_one_placeholder_on_sqlite() {
  let node = SharedBind::new("candidate");
  let files = SharedBindList::new(["a", "b", "c"]);

  let (sql, params) = both_directions(&node, &files).to_sql_fragment_for(1, Dialect::Sqlite);

  assert_eq!(
    sql,
    concat!(
      r#"(("edges"."src_id" = ?1 AND "edges"."dst_id" IN (?2, ?3, ?4))"#,
      r#" OR ("edges"."dst_id" = ?1 AND "edges"."src_id" IN (?2, ?3, ?4)))"#
    )
  );
  assert_eq!(
    params,
    vec![text("candidate"), text("a"), text("b"), text("c")]
  );
}

#[test]
fn the_same_predicate_renders_dollar_placeholders_on_postgres() {
  let node = SharedBind::new("candidate");
  let files = SharedBindList::new(["a", "b", "c"]);

  let (sql, params) = both_directions(&node, &files).to_sql_fragment_for(1, Dialect::Postgres);

  assert_eq!(
    sql,
    concat!(
      r#"(("edges"."src_id" = $1 AND "edges"."dst_id" IN ($2, $3, $4))"#,
      r#" OR ("edges"."dst_id" = $1 AND "edges"."src_id" IN ($2, $3, $4)))"#
    )
  );
  assert_eq!(
    params,
    vec![text("candidate"), text("a"), text("b"), text("c")]
  );
}

#[test]
fn every_rendered_index_is_backed_by_a_bound_value() {
  let node = SharedBind::new("candidate");
  let files = SharedBindList::new(["a", "b", "c"]);

  let (sql, params) = both_directions(&node, &files).to_sql_fragment_for(1, Dialect::Sqlite);
  let indices = sqlite_indices(&sql);

  // Eight placeholders are written, but only four values are bound: the
  // shared candidate and the shared list each appear in both orientations.
  assert_eq!(indices.len(), 8);
  assert_eq!(params.len(), 4);
  let mut distinct: Vec<usize> = indices.clone();
  distinct.sort_unstable();
  distinct.dedup();
  assert_eq!(distinct, vec![1, 2, 3, 4]);
  assert!(indices.iter().all(|index| *index <= params.len()));
}

#[test]
fn an_owned_predicate_beside_a_shared_one_keeps_binding_per_occurrence() {
  let node = SharedBind::new("candidate");

  let (sql, params) = REL
    .eq("co_changed")
    .and(SRC_ID.eq_shared(&node))
    .and(DST_ID.eq_shared(&node))
    .and(REL.eq("co_changed"))
    .to_sql_fragment_for(1, Dialect::Sqlite);

  assert_eq!(
    sql,
    concat!(
      r#"((("edges"."rel" = ?1 AND "edges"."src_id" = ?2)"#,
      r#" AND "edges"."dst_id" = ?2) AND "edges"."rel" = ?3)"#
    )
  );
  assert_eq!(
    params,
    vec![text("co_changed"), text("candidate"), text("co_changed")]
  );
}

#[test]
fn a_fragment_spliced_at_an_offset_numbers_its_shared_index_from_there() {
  let node = SharedBind::new("candidate");
  let files = SharedBindList::new(["a", "b"]);

  let (sql, params) = both_directions(&node, &files).to_sql_fragment_for(4, Dialect::Sqlite);

  assert_eq!(
    sql,
    concat!(
      r#"(("edges"."src_id" = ?4 AND "edges"."dst_id" IN (?5, ?6))"#,
      r#" OR ("edges"."dst_id" = ?4 AND "edges"."src_id" IN (?5, ?6)))"#
    )
  );
  assert_eq!(params, vec![text("candidate"), text("a"), text("b")]);
}

#[test]
fn a_negated_shared_predicate_renders_the_same_indices() {
  let node = SharedBind::new("candidate");
  let files = SharedBindList::new(["a", "b"]);

  let (sql, params) = SRC_ID
    .ne_shared(&node)
    .and(DST_ID.not_in_shared(&files))
    .or(DST_ID.eq_shared(&node).and(SRC_ID.in_shared(&files)))
    .to_sql_fragment_for(1, Dialect::Sqlite);

  assert_eq!(
    sql,
    concat!(
      r#"(("edges"."src_id" != ?1 AND "edges"."dst_id" NOT IN (?2, ?3))"#,
      r#" OR ("edges"."dst_id" = ?1 AND "edges"."src_id" IN (?2, ?3)))"#
    )
  );
  assert_eq!(params.len(), 3);
  assert_eq!(params.first(), Some(&Value::Text("candidate".to_owned())));
}
