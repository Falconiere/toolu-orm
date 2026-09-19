//! How a shared placeholder meets the two things that number themselves: a
//! raw fragment's bare `?`, and a `CASE` that binds mid-tree.

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::{Expr, Scalar, SharedBind, SharedBindList};
use toolu_orm_core::query_column::{CommonOps, NumericOps, SharedOps};
use toolu_orm_core::value::Value;

use crate::fixtures::{text, DST_ID, REL, SRC_ID, WEIGHT};

#[test]
fn a_raw_fragment_after_a_reuse_takes_the_next_unused_index() {
  let node = SharedBind::new("candidate");

  let (sql, params) = SRC_ID
    .eq_shared(&node)
    .and(DST_ID.eq_shared(&node))
    .and(Expr::raw(
      r#""edges"."weight" > ?"#,
      vec![Value::Integer(5)],
    ))
    .to_sql_fragment_for(1, Dialect::Sqlite);

  // The reuse pushed nothing, so the raw fragment numbers ?2, not ?3.
  assert_eq!(
    sql,
    concat!(
      r#"(("edges"."src_id" = ?1 AND "edges"."dst_id" = ?1)"#,
      r#" AND "edges"."weight" > ?2)"#
    )
  );
  assert_eq!(params, vec![text("candidate"), Value::Integer(5)]);
}

#[test]
fn a_raw_fragment_before_a_reuse_still_owns_the_low_indices() {
  let node = SharedBind::new("candidate");

  let (sql, params) = Expr::raw(r#""edges"."weight" > ?"#, vec![Value::Integer(5)])
    .and(SRC_ID.eq_shared(&node))
    .and(DST_ID.eq_shared(&node))
    .to_sql_fragment_for(1, Dialect::Sqlite);

  assert_eq!(
    sql,
    concat!(
      r#"(("edges"."weight" > ?1 AND "edges"."src_id" = ?2)"#,
      r#" AND "edges"."dst_id" = ?2)"#
    )
  );
  assert_eq!(params, vec![Value::Integer(5), text("candidate")]);
}

#[test]
fn a_shared_scalar_inside_a_case_shares_with_the_predicate_around_it() {
  let node = SharedBind::new("candidate");

  let label = Scalar::case_when(SRC_ID.eq_shared(&node), Scalar::shared(&node))
    .otherwise(Scalar::bind("other"));

  let (sql, params) = label
    .eq(Scalar::shared(&node))
    .and(DST_ID.eq_shared(&node))
    .to_sql_fragment_for(1, Dialect::Sqlite);

  assert_eq!(
    sql,
    concat!(
      r#"(CASE WHEN "edges"."src_id" = ?1 THEN ?1 ELSE ?2 END = ?1"#,
      r#" AND "edges"."dst_id" = ?1)"#
    )
  );
  assert_eq!(params, vec![text("candidate"), text("other")]);
}

#[test]
fn a_shared_scalar_composes_with_arithmetic_and_binds_once() {
  let bump = SharedBind::new(3);

  let (sql, params) = (Scalar::col(&WEIGHT) + Scalar::shared(&bump))
    .gt(Scalar::shared(&bump))
    .to_sql_fragment_for(1, Dialect::Sqlite);

  assert_eq!(sql, r#"("edges"."weight" + ?1) > ?1"#);
  assert_eq!(params, vec![Value::Integer(3)]);
}

#[test]
fn a_shared_list_and_an_owned_list_of_the_same_values_stay_apart() {
  let files = SharedBindList::new(["a", "b"]);
  let owned = vec![text("a"), text("b")];

  let (sql, params) = SRC_ID
    .in_shared(&files)
    .and(DST_ID.in_list(&owned))
    .and(SRC_ID.in_shared(&files))
    .to_sql_fragment_for(1, Dialect::Sqlite);

  assert_eq!(
    sql,
    concat!(
      r#"(("edges"."src_id" IN (?1, ?2) AND "edges"."dst_id" IN (?3, ?4))"#,
      r#" AND "edges"."src_id" IN (?1, ?2))"#
    )
  );
  assert_eq!(params.len(), 4);
}

#[test]
fn a_between_beside_a_reuse_keeps_its_own_two_placeholders() {
  let node = SharedBind::new("candidate");

  let (sql, params) = SRC_ID
    .eq_shared(&node)
    .and(WEIGHT.between(1, 9))
    .and(DST_ID.eq_shared(&node))
    .to_sql_fragment_for(1, Dialect::Sqlite);

  assert_eq!(
    sql,
    concat!(
      r#"(("edges"."src_id" = ?1 AND "edges"."weight" BETWEEN ?2 AND ?3)"#,
      r#" AND "edges"."dst_id" = ?1)"#
    )
  );
  assert_eq!(
    params,
    vec![text("candidate"), Value::Integer(1), Value::Integer(9)]
  );
}

#[test]
fn a_literal_index_in_a_raw_fragment_addresses_its_own_values() {
  let node = SharedBind::new("candidate");

  // Since issue #131 `?1` names the *fragment's* first value, not the
  // statement's: this fragment renders third, so it says ?3 and compares
  // against the value it bound. It still cannot name a handle's placeholder —
  // the handle took ?2, and no index the fragment can write means "that one".
  let (sql, params) = REL
    .eq("co_changed")
    .and(SRC_ID.eq_shared(&node))
    .and(Expr::raw(r#""edges"."dst_id" = ?1"#, vec![text("other")]))
    .and(DST_ID.eq_shared(&node))
    .to_sql_fragment_for(1, Dialect::Sqlite);

  assert_eq!(
    sql,
    concat!(
      r#"((("edges"."rel" = ?1 AND "edges"."src_id" = ?2)"#,
      r#" AND "edges"."dst_id" = ?3) AND "edges"."dst_id" = ?2)"#
    )
  );
  assert_eq!(
    params,
    vec![text("co_changed"), text("candidate"), text("other")]
  );
  // `Scalar::shared` is the supported route to a handle's index: it renders
  // the handle's own placeholder without the caller knowing what it is.
  let (shared_sql, _) = Scalar::col(&DST_ID)
    .eq(Scalar::shared(&node))
    .to_sql_fragment_for(1, Dialect::Sqlite);
  assert_eq!(shared_sql, r#""edges"."dst_id" = ?1"#);
}

#[test]
fn a_numbered_raw_fragment_after_a_reuse_takes_the_next_unused_index() {
  let node = SharedBind::new("candidate");

  // The second `eq_shared` binds nothing, so the fragment's base is 2 and its
  // own `?1` — written twice for one value — renders ?2 at both occurrences.
  let (sql, params) = SRC_ID
    .eq_shared(&node)
    .and(DST_ID.eq_shared(&node))
    .and(Expr::raw(
      r#"("edges"."weight" > ?1 OR "edges"."weight" < -?1)"#,
      vec![Value::Integer(5)],
    ))
    .to_sql_fragment_for(1, Dialect::Sqlite);

  assert_eq!(
    sql,
    concat!(
      r#"(("edges"."src_id" = ?1 AND "edges"."dst_id" = ?1)"#,
      r#" AND ("edges"."weight" > ?2 OR "edges"."weight" < -?2))"#
    )
  );
  assert_eq!(params, vec![text("candidate"), Value::Integer(5)]);
}
