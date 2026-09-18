//! CTEs, set operations, subqueries and table-valued sources compose with
//! `toolu-orm` as the only dependency.
//!
//! Like its sibling binaries this package depends on `toolu-orm` alone, so
//! `toolu_orm_core` and `toolu_orm_query` are absent from the extern prelude
//! and every path here is written through the facade.

use toolu_orm::core::alias::TableRef;
use toolu_orm::core::column::{Integer, Text};
use toolu_orm::core::dialect::Dialect;
use toolu_orm::core::expr::{Expr, OrderBy, Scalar};
use toolu_orm::core::query_column::{CommonOps, NumericOps};
use toolu_orm::core::value::Value;
use toolu_orm::query::select::{Cte, SelectBuilder};
use toolu_orm::table;

#[table(name = "facade_edges")]
pub struct FacadeEdge {
  #[column(primary_key)]
  pub id: Text,
  #[column(not_null)]
  pub src_id: Text,
  #[column(not_null)]
  pub dst_id: Text,
  pub depth: Integer,
}

/// A recursive CTE built entirely through the facade, rendered for an explicit
/// dialect because `to_sql()` follows `Dialect::CURRENT`.
#[test]
fn a_recursive_cte_composes_through_the_facade() {
  let anchor = FacadeEdge::select()
    .columns_raw(&["dst_id"])
    .filter(facade_edges::src_id.eq("root"));
  let step = FacadeEdge::select()
    .columns_raw(&["dst_id"])
    .filter(facade_edges::depth.lt(3));
  let walk = Cte::new("walk", anchor.union(step))
    .columns(&["id"])
    .recursive();

  let (sql, params) = SelectBuilder::from_table(walk.table_ref())
    .columns_raw(&["id"])
    .with(walk)
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"WITH RECURSIVE "walk"("id") AS (SELECT "dst_id" FROM "facade_edges" "#.to_owned()
      + r#"WHERE "facade_edges"."src_id" = ?1 UNION SELECT "dst_id" FROM "facade_edges" "#
      + r#"WHERE "facade_edges"."depth" < ?2) SELECT "id" FROM "walk""#
  );
  assert_eq!(
    params,
    vec![Value::Text("root".to_owned()), Value::Integer(3)]
  );
}

#[test]
fn a_union_with_an_output_ordering_composes_through_the_facade() {
  let (sql, params) = FacadeEdge::select()
    .column_as(&facade_edges::dst_id, "id")
    .filter(facade_edges::src_id.eq("a"))
    .union_all(
      FacadeEdge::select()
        .column_as(&facade_edges::src_id, "id")
        .filter(facade_edges::dst_id.eq("b")),
    )
    .order_by(OrderBy::alias_asc("id"))
    .to_sql_for(Dialect::Sqlite);

  assert!(sql.contains(" UNION ALL SELECT "), "got: {sql}");
  assert!(sql.ends_with(r#"ORDER BY "id" ASC"#), "got: {sql}");
  assert_eq!(
    params,
    vec![Value::Text("a".to_owned()), Value::Text("b".to_owned())]
  );
}

#[test]
fn subquery_predicates_compose_through_the_facade() {
  let inner = || {
    FacadeEdge::select()
      .columns_qualified(&[&facade_edges::dst_id])
      .filter(facade_edges::depth.gt(1))
  };

  let (in_sql, in_params) = FacadeEdge::select()
    .columns_raw(&["id"])
    .filter(Scalar::col(&facade_edges::src_id).in_subquery(inner()))
    .to_sql_for(Dialect::Sqlite);
  let (exists_sql, _) = FacadeEdge::select()
    .columns_raw(&["id"])
    .filter(Expr::not_exists(inner()))
    .to_sql_for(Dialect::Sqlite);
  let (scalar_sql, _) = FacadeEdge::select()
    .column_scalar(Scalar::subquery(inner()), "first")
    .to_sql_for(Dialect::Sqlite);

  assert!(
    in_sql.contains(r#""facade_edges"."src_id" IN (SELECT "#),
    "got: {in_sql}"
  );
  assert_eq!(in_params, vec![Value::Integer(1)]);
  assert!(
    exists_sql.contains("WHERE NOT EXISTS (SELECT "),
    "got: {exists_sql}"
  );
  assert!(
    scalar_sql.starts_with(r#"SELECT (SELECT "facade_edges"."dst_id""#),
    "got: {scalar_sql}"
  );
}

#[test]
fn a_table_valued_source_composes_through_the_facade() {
  let seeds = TableRef::function("json_each", vec![Value::Text("[1]".to_owned())])
    .expect("json_each is a bare identifier")
    .with_alias("seeds");

  let (sql, params) = SelectBuilder::from_table(&seeds)
    .columns_raw(&["value"])
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(sql, r#"SELECT "value" FROM json_each(?1) AS "seeds""#);
  assert_eq!(params, vec![Value::Text("[1]".to_owned())]);
}
