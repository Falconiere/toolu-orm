//! A `SelectSource` implemented *outside* the query crate, which does not
//! override `to_select_sql_into`.
//!
//! This is the one place sharing degrades, and it degrades safely: the nested
//! statement renders through its own ledger, so a handle used on both sides
//! binds twice. The SQL stays correct, the values stay correct, and — the
//! property that actually matters — no placeholder ever names an index nothing
//! bound.

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::{Expr, SelectSource, SharedBind};
use toolu_orm_core::query_column::SharedOps;
use toolu_orm_core::value::Value;

use crate::fixtures::{sqlite_indices, text, DST_ID, SRC_ID};

/// A minimal foreign statement, built the only way a foreign crate can: from
/// the public fragment API, which hands back a self-contained `(sql, params)`
/// pair. It implements only the required trait method.
struct ForeignSelect {
  predicate: Expr,
}

impl SelectSource for ForeignSelect {
  fn to_select_sql_for(&self, start: usize, dialect: Dialect) -> (String, Vec<Value>) {
    let (fragment, params) = self.predicate.to_sql_fragment_for(start, dialect);
    (format!(r#"SELECT 1 FROM "t" WHERE {fragment}"#), params)
  }
}

#[test]
fn a_foreign_source_still_numbers_from_where_its_siblings_left_off() {
  let node = SharedBind::new("candidate");
  let foreign = ForeignSelect {
    predicate: SRC_ID.eq_shared(&SharedBind::new("k1")),
  };

  let (sql, params) = SRC_ID
    .eq_shared(&node)
    .and(Expr::exists(foreign))
    .and(DST_ID.eq_shared(&node))
    .to_sql_fragment_for(1, Dialect::Sqlite);

  assert_eq!(
    sql,
    concat!(
      r#"(("edges"."src_id" = ?1 AND EXISTS (SELECT 1 FROM "t" WHERE "edges"."src_id" = ?2))"#,
      r#" AND "edges"."dst_id" = ?1)"#
    )
  );
  assert_eq!(params, vec![text("candidate"), text("k1")]);
}

#[test]
fn a_handle_inside_a_foreign_source_binds_again_rather_than_taking_a_wrong_index() {
  let node = SharedBind::new("candidate");
  // The *same handle* is used inside the foreign statement and outside it.
  let foreign = ForeignSelect {
    predicate: SRC_ID.eq_shared(&node),
  };

  let (sql, params) = SRC_ID
    .eq_shared(&node)
    .and(Expr::exists(foreign))
    .and(DST_ID.eq_shared(&node))
    .to_sql_fragment_for(1, Dialect::Sqlite);

  // Two parameters, not one: the foreign statement renders through its own
  // ledger, so sharing stops at a boundary this crate does not control. The
  // two occurrences *outside* it still share ?1.
  assert_eq!(
    sql,
    concat!(
      r#"(("edges"."src_id" = ?1 AND EXISTS (SELECT 1 FROM "t" WHERE "edges"."src_id" = ?2))"#,
      r#" AND "edges"."dst_id" = ?1)"#
    )
  );
  assert_eq!(params, vec![text("candidate"), text("candidate")]);

  // The property that must never degrade: every index written is backed by a
  // bound value, and every bound value is referenced.
  let mut distinct = sqlite_indices(&sql);
  distinct.sort_unstable();
  distinct.dedup();
  assert_eq!(distinct, vec![1, 2]);
}
