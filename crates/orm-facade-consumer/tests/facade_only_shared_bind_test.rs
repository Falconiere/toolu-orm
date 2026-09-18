//! Reusable bound parameters compose with `toolu-orm` as the only dependency.
//!
//! Like its sibling binaries this package depends on `toolu-orm` alone, so
//! `toolu_orm_core` is absent from the extern prelude and every path here is
//! written through the facade.

use toolu_orm::core::column::{Integer, Text};
use toolu_orm::core::dialect::Dialect;
use toolu_orm::core::expr::{Scalar, SharedBind, SharedBindList};
use toolu_orm::core::query_column::{CommonOps, SharedOps};
use toolu_orm::core::value::Value;
use toolu_orm::table;

#[table(name = "facade_edges")]
pub struct FacadeEdge {
  #[column(primary_key)]
  pub id: Text,
  #[column(not_null)]
  pub src_id: Text,
  #[column(not_null)]
  pub dst_id: Text,
  pub weight: Integer,
}

/// Rendered for an explicit dialect: `to_sql()` follows `Dialect::CURRENT`,
/// which is Postgres (`$N`) in the postgres lane and SQLite (`?N`) elsewhere,
/// and this asserts the placeholder text.
#[test]
fn a_reused_binding_takes_one_placeholder_through_the_facade() {
  let candidate = SharedBind::new("file:r:candidate.rs");
  let files = SharedBindList::new(["a.rs", "b.rs"]);

  let (sql, params) = FacadeEdge::select()
    .columns_raw(&["id"])
    .filter(facade_edges::src_id.eq("x"))
    .filter(
      facade_edges::src_id
        .eq_shared(&candidate)
        .and(facade_edges::dst_id.in_shared(&files))
        .or(
          facade_edges::dst_id
            .eq_shared(&candidate)
            .and(facade_edges::src_id.in_shared(&files)),
        ),
    )
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    concat!(
      r#"SELECT "id" FROM "facade_edges" WHERE "facade_edges"."src_id" = ?1"#,
      r#" AND (("facade_edges"."src_id" = ?2 AND "facade_edges"."dst_id" IN (?3, ?4))"#,
      r#" OR ("facade_edges"."dst_id" = ?2 AND "facade_edges"."src_id" IN (?3, ?4)))"#
    )
  );
  assert_eq!(
    params,
    vec![
      Value::Text("x".to_owned()),
      Value::Text("file:r:candidate.rs".to_owned()),
      Value::Text("a.rs".to_owned()),
      Value::Text("b.rs".to_owned()),
    ]
  );
}

#[test]
fn the_same_predicate_renders_dollar_placeholders_through_the_facade() {
  let candidate = SharedBind::new("file:r:candidate.rs");

  let (sql, params) = FacadeEdge::select()
    .column_scalar(Scalar::shared(&candidate), "asked_about")
    .filter(facade_edges::src_id.eq_shared(&candidate))
    .filter(facade_edges::dst_id.ne_shared(&candidate))
    .to_sql_for(Dialect::Postgres);

  assert_eq!(
    sql,
    concat!(
      r#"SELECT $1 AS "asked_about" FROM "facade_edges""#,
      r#" WHERE "facade_edges"."src_id" = $1 AND "facade_edges"."dst_id" != $1"#
    )
  );
  assert_eq!(params, vec![Value::Text("file:r:candidate.rs".to_owned())]);
}

#[test]
fn two_handles_over_equal_values_stay_independent_through_the_facade() {
  let left = SharedBind::new("same");
  let right = SharedBind::new("same");

  let (sql, params) = FacadeEdge::select()
    .columns_raw(&["id"])
    .filter(facade_edges::src_id.eq_shared(&left))
    .filter(facade_edges::dst_id.eq_shared(&right))
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    concat!(
      r#"SELECT "id" FROM "facade_edges""#,
      r#" WHERE "facade_edges"."src_id" = ?1 AND "facade_edges"."dst_id" = ?2"#
    )
  );
  assert_eq!(params.len(), 2);
}
