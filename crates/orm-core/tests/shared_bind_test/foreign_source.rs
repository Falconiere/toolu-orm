//! A `SelectSource` implemented *outside* the query crate, which does not
//! override `to_select_sql_into`.
//!
//! This is the one place sharing degrades, and it degrades safely: the nested
//! statement renders with its own ledger, so a handle used on both sides binds
//! twice. The SQL stays correct, the values stay correct, and — the property
//! that actually matters — no placeholder ever names an index nothing bound.

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::{Expr, SelectSource, SharedBind};
use toolu_orm_core::query_column::SharedOps;
use toolu_orm_core::value::Value;

use crate::fixtures::{sqlite_indices, text, DST_ID, SRC_ID};

/// A minimal foreign statement: `SELECT 1 FROM "t" WHERE "k" = ?N`, binding
/// whatever it was built with. It implements only the required method.
struct ForeignSelect {
  key: Value,
}

impl SelectSource for ForeignSelect {
  fn to_select_sql_for(&self, start: usize, dialect: Dialect) -> (String, Vec<Value>) {
    (
      format!(r#"SELECT 1 FROM "t" WHERE "k" = {}"#, dialect.param(start)),
      vec![self.key.clone()],
    )
  }
}

#[test]
fn a_foreign_source_still_numbers_from_where_its_siblings_left_off() {
  let node = SharedBind::new("candidate");

  let (sql, params) = SRC_ID
    .eq_shared(&node)
    .and(Expr::exists(ForeignSelect { key: text("k1") }))
    .and(DST_ID.eq_shared(&node))
    .to_sql_fragment_for(1, Dialect::Sqlite);

  assert_eq!(
    sql,
    concat!(
      r#"(("edges"."src_id" = ?1 AND EXISTS (SELECT 1 FROM "t" WHERE "k" = ?2))"#,
      r#" AND "edges"."dst_id" = ?1)"#
    )
  );
  assert_eq!(params, vec![text("candidate"), text("k1")]);
}

#[test]
fn a_handle_inside_a_foreign_source_binds_again_rather_than_taking_a_wrong_index() {
  let node = SharedBind::new("candidate");
  // The foreign statement carries the same *value*, but it renders through its
  // own ledger, so it allocates its own placeholder.
  let foreign = ForeignSelect {
    key: text("candidate"),
  };

  let (sql, params) = SRC_ID
    .eq_shared(&node)
    .and(Expr::exists(foreign))
    .and(DST_ID.eq_shared(&node))
    .to_sql_fragment_for(1, Dialect::Sqlite);

  // Two parameters, not one: sharing is best effort across a boundary this
  // crate does not control. Correct SQL, correct values, one extra bind.
  assert_eq!(params, vec![text("candidate"), text("candidate")]);

  // The property that must never degrade: every index written is backed by a
  // bound value, and every bound value is referenced.
  let mut distinct = sqlite_indices(&sql);
  distinct.sort_unstable();
  distinct.dedup();
  assert_eq!(distinct, vec![1, 2]);
}
