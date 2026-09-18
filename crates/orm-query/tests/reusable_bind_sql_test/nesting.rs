//! Sharing across a statement boundary: a CTE body, a `UNION` arm, an
//! `IN (SELECT …)`, a scalar subquery, and the derived-table count wrap.

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::{OrderBy, Scalar, SharedBind};
use toolu_orm_core::query_column::{CommonOps, SharedOps};
use toolu_orm_core::value::Value;
use toolu_orm_query::select::{Cte, SelectBuilder};

use crate::seed::{DST_ID, REL, SRC_ID, WEIGHT};

/// Every placeholder index the SQLite rendering names, in written order.
fn indices(sql: &str) -> Vec<usize> {
  let mut found = Vec::new();
  let mut chars = sql.chars().peekable();
  while let Some(ch) = chars.next() {
    if ch != '?' {
      continue;
    }
    let mut digits = String::new();
    while chars.peek().is_some_and(char::is_ascii_digit) {
      if let Some(digit) = chars.next() {
        digits.push(digit);
      }
    }
    if let Ok(index) = digits.parse::<usize>() {
      found.push(index);
    }
  }
  found
}

/// Asserts that every index written is backed by a bound value and that every
/// bound value is referenced — the invariant a wrong shared index would break.
fn assert_indices_cover_params(sql: &str, params: &[Value]) {
  let mut distinct = indices(sql);
  distinct.sort_unstable();
  distinct.dedup();
  assert_eq!(
    distinct,
    (1..=params.len()).collect::<Vec<usize>>(),
    "placeholders {distinct:?} do not cover 1..={} in {sql}",
    params.len()
  );
}

#[test]
fn a_handle_first_used_in_a_cte_body_is_reused_by_the_outer_where() {
  let node = SharedBind::new("candidate");
  let body = SelectBuilder::new("edges")
    .columns_qualified(&[&SRC_ID])
    .filter(SRC_ID.eq_shared(&node));

  let (sql, params) = SelectBuilder::new("edges")
    .columns_qualified(&[&DST_ID])
    .with(Cte::new("seen", body))
    .filter(DST_ID.eq_shared(&node))
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    concat!(
      r#"WITH "seen" AS (SELECT "edges"."src_id" FROM "edges" WHERE "edges"."src_id" = ?1) "#,
      r#"SELECT "edges"."dst_id" FROM "edges" WHERE "edges"."dst_id" = ?1"#
    )
  );
  assert_eq!(params, vec![Value::Text("candidate".to_owned())]);
  assert_indices_cover_params(&sql, &params);
}

#[test]
fn a_handle_first_used_outside_is_reused_inside_a_subquery() {
  let node = SharedBind::new("candidate");
  let inner = SelectBuilder::new("edges")
    .columns_qualified(&[&DST_ID])
    .filter(DST_ID.eq_shared(&node));

  let (sql, params) = SelectBuilder::new("edges")
    .columns_qualified(&[&SRC_ID])
    .filter(SRC_ID.eq_shared(&node))
    .filter(Scalar::col(&DST_ID).in_subquery(inner))
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    concat!(
      r#"SELECT "edges"."src_id" FROM "edges" WHERE "edges"."src_id" = ?1 AND "#,
      r#""edges"."dst_id" IN (SELECT "edges"."dst_id" FROM "edges" WHERE "edges"."dst_id" = ?1)"#
    )
  );
  assert_eq!(params, vec![Value::Text("candidate".to_owned())]);
  assert_indices_cover_params(&sql, &params);
}

#[test]
fn a_handle_first_used_inside_a_subquery_is_reused_by_a_later_clause() {
  let node = SharedBind::new("candidate");
  let inner = SelectBuilder::new("edges")
    .columns_qualified(&[&DST_ID])
    .filter(DST_ID.eq_shared(&node));

  let (sql, params) = SelectBuilder::new("edges")
    .columns_qualified(&[&SRC_ID])
    .filter(REL.eq("co_changed"))
    .filter(Scalar::col(&SRC_ID).in_subquery(inner))
    .filter(DST_ID.eq_shared(&node))
    .to_sql_for(Dialect::Sqlite);

  // The subquery allocated ?2 for the handle; the filter after it reuses ?2.
  assert_eq!(
    sql,
    concat!(
      r#"SELECT "edges"."src_id" FROM "edges" WHERE "edges"."rel" = ?1 AND "#,
      r#""edges"."src_id" IN (SELECT "edges"."dst_id" FROM "edges" WHERE "edges"."dst_id" = ?2)"#,
      r#" AND "edges"."dst_id" = ?2"#
    )
  );
  assert_eq!(
    params,
    vec![
      Value::Text("co_changed".to_owned()),
      Value::Text("candidate".to_owned())
    ]
  );
  assert_indices_cover_params(&sql, &params);
}

#[test]
fn union_arms_share_one_placeholder_with_the_arm_before_them() {
  let node = SharedBind::new("candidate");
  let arm = SelectBuilder::new("edges")
    .column_as(&DST_ID, "id")
    .filter(DST_ID.eq_shared(&node));

  let (sql, params) = SelectBuilder::new("edges")
    .column_as(&SRC_ID, "id")
    .filter(SRC_ID.eq_shared(&node))
    .union(arm)
    .order_by(OrderBy::alias_asc("id"))
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    concat!(
      r#"SELECT "edges"."src_id" AS "id" FROM "edges" WHERE "edges"."src_id" = ?1"#,
      r#" UNION SELECT "edges"."dst_id" AS "id" FROM "edges" WHERE "edges"."dst_id" = ?1"#,
      r#" ORDER BY "id" ASC"#
    )
  );
  assert_eq!(params, vec![Value::Text("candidate".to_owned())]);
}

#[test]
fn the_derived_table_count_wrap_numbers_from_one_and_shares_inside_itself() {
  let node = SharedBind::new("candidate");

  let (sql, params) = SelectBuilder::new("edges")
    .columns_qualified(&[&SRC_ID])
    .distinct()
    .filter(SRC_ID.eq_shared(&node))
    .filter(DST_ID.eq_shared(&node))
    .to_count_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    concat!(
      r#"SELECT COUNT(*) FROM (SELECT DISTINCT "edges"."src_id" FROM "edges""#,
      r#" WHERE "edges"."src_id" = ?1 AND "edges"."dst_id" = ?1) AS "toolu_count""#
    )
  );
  assert_eq!(params, vec![Value::Text("candidate".to_owned())]);
  assert_indices_cover_params(&sql, &params);
}

#[test]
fn a_scalar_subquery_projection_shares_with_the_where_that_follows_it() {
  let node = SharedBind::new("candidate");
  let inner = SelectBuilder::new("edges")
    .column_scalar(Scalar::max(Scalar::col(&WEIGHT)), "top")
    .filter(SRC_ID.eq_shared(&node));

  let (sql, params) = SelectBuilder::new("edges")
    .column_scalar(Scalar::subquery(inner), "top")
    .filter(DST_ID.eq_shared(&node))
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    concat!(
      r#"SELECT (SELECT MAX("edges"."weight") AS "top" FROM "edges""#,
      r#" WHERE "edges"."src_id" = ?1) AS "top" FROM "edges""#,
      r#" WHERE "edges"."dst_id" = ?1"#
    )
  );
  assert_eq!(params, vec![Value::Text("candidate".to_owned())]);
  assert_indices_cover_params(&sql, &params);
}
