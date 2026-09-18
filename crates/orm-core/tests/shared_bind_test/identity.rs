//! Identity is the handle, never the value.

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::{Scalar, SharedBind, SharedBindList};
use toolu_orm_core::query_column::{CommonOps, SharedOps};
use toolu_orm_core::value::Value;

use crate::fixtures::{text, DST_ID, SRC_ID};

#[test]
fn two_handles_holding_equal_values_stay_independent() {
  let left = SharedBind::new("file:a.rs");
  let right = SharedBind::new("file:a.rs");

  let (sql, params) = SRC_ID
    .eq_shared(&left)
    .and(DST_ID.eq_shared(&right))
    .to_sql_fragment_for(1, Dialect::Sqlite);

  assert_eq!(sql, r#"("edges"."src_id" = ?1 AND "edges"."dst_id" = ?2)"#);
  assert_eq!(params, vec![text("file:a.rs"), text("file:a.rs")]);
}

#[test]
fn a_cloned_handle_is_the_same_binding() {
  let node = SharedBind::new("file:a.rs");
  let same = node.clone();

  let (sql, params) = SRC_ID
    .eq_shared(&node)
    .and(DST_ID.eq_shared(&same))
    .to_sql_fragment_for(1, Dialect::Sqlite);

  assert_eq!(sql, r#"("edges"."src_id" = ?1 AND "edges"."dst_id" = ?1)"#);
  assert_eq!(params, vec![text("file:a.rs")]);
}

#[test]
fn two_list_handles_holding_equal_values_stay_independent() {
  let left = SharedBindList::new(["a", "b"]);
  let right = SharedBindList::new(["a", "b"]);

  let (sql, params) = SRC_ID
    .in_shared(&left)
    .or(DST_ID.in_shared(&right))
    .to_sql_fragment_for(1, Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"("edges"."src_id" IN (?1, ?2) OR "edges"."dst_id" IN (?3, ?4))"#
  );
  assert_eq!(params.len(), 4);
}

#[test]
fn a_handle_used_once_binds_exactly_what_the_owned_form_binds() {
  let node = SharedBind::new("file:a.rs");

  let shared = SRC_ID
    .eq_shared(&node)
    .to_sql_fragment_for(1, Dialect::Sqlite);
  let owned = SRC_ID
    .eq("file:a.rs")
    .to_sql_fragment_for(1, Dialect::Sqlite);

  assert_eq!(shared, owned);
}

#[test]
fn rendering_the_same_expression_twice_is_idempotent() {
  let node = SharedBind::new("file:a.rs");
  let files = SharedBindList::new(["x", "y"]);
  let expr = SRC_ID
    .eq_shared(&node)
    .and(DST_ID.in_shared(&files))
    .or(DST_ID.eq_shared(&node).and(SRC_ID.in_shared(&files)));

  let first = expr.to_sql_fragment_for(1, Dialect::Sqlite);
  let second = expr.to_sql_fragment_for(1, Dialect::Sqlite);

  assert_eq!(first, second);
  assert_eq!(first.1.len(), 3);
}

#[test]
fn an_empty_list_handle_renders_the_constant_at_every_occurrence() {
  let none = SharedBindList::new(Vec::<Value>::new());
  assert!(none.is_empty());
  assert_eq!(none.len(), 0);

  let (sql, params) = SRC_ID
    .in_shared(&none)
    .and(DST_ID.in_shared(&none))
    .or(SRC_ID.not_in_shared(&none))
    .to_sql_fragment_for(1, Dialect::Sqlite);

  assert_eq!(sql, "((1 = 0 AND 1 = 0) OR 1 = 1)");
  assert_eq!(params, Vec::<Value>::new());
}

#[test]
fn a_handle_exposes_the_payload_it_binds() {
  let node = SharedBind::new("file:a.rs");
  let files = SharedBindList::new(["x", "y"]);

  assert_eq!(node.value(), &text("file:a.rs"));
  assert_eq!(files.values(), &[text("x"), text("y")]);
  assert_eq!(files.len(), 2);
  assert!(!files.is_empty());
}

#[test]
fn the_same_handle_renders_in_scalar_and_predicate_position() {
  let node = SharedBind::new("file:a.rs");

  let (sql, params) = Scalar::col(&SRC_ID)
    .eq(Scalar::shared(&node))
    .and(DST_ID.eq_shared(&node))
    .to_sql_fragment_for(1, Dialect::Sqlite);

  assert_eq!(sql, r#"("edges"."src_id" = ?1 AND "edges"."dst_id" = ?1)"#);
  assert_eq!(params, vec![text("file:a.rs")]);
}
