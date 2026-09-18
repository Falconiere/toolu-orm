//! Column, bound-value and raw-SQL leaves.

use toolu_orm_core::column::{Integer, Text};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::Scalar;
use toolu_orm_core::query_column::Column;
use toolu_orm_core::value::Value;

const BODY: Column<Text> = Column::new("memories", "body");
const HITS: Column<Integer> = Column::new("memories", "access_count");

#[test]
fn a_column_leaf_renders_qualified_and_binds_nothing() {
  let (sql, params) = Scalar::col(&BODY).to_sql_fragment_for(1, Dialect::Sqlite);
  assert_eq!(sql, r#""memories"."body""#);
  assert!(params.is_empty());

  let (pg_sql, pg_params) = Scalar::col(&HITS).to_sql_fragment_for(7, Dialect::Postgres);
  assert_eq!(pg_sql, r#""memories"."access_count""#);
  assert!(pg_params.is_empty());
}

#[test]
fn a_bound_leaf_takes_the_offset_it_is_rendered_at() {
  let (sqlite, sqlite_params) = Scalar::bind(41).to_sql_fragment_for(1, Dialect::Sqlite);
  assert_eq!(sqlite, "?1");
  assert_eq!(sqlite_params, vec![Value::Integer(41)]);

  let (postgres, postgres_params) = Scalar::bind("x").to_sql_fragment_for(4, Dialect::Postgres);
  assert_eq!(postgres, "$4");
  assert_eq!(postgres_params, vec![Value::Text("x".to_owned())]);
}

#[test]
fn a_sql_leaf_passes_its_text_through_unchanged() {
  let (sql, params) = Scalar::sql(r#"bm25("memory_fts")"#).to_sql_fragment_for(3, Dialect::Sqlite);
  assert_eq!(sql, r#"bm25("memory_fts")"#);
  assert!(params.is_empty());
}

#[test]
fn a_raw_leaf_numbers_its_placeholders_from_the_offset() {
  let raw = Scalar::raw(
    "substr(?, 1, ?)",
    vec![Value::Text("abc".to_owned()), 2.into()],
  );

  let (sqlite, sqlite_params) = raw.to_sql_fragment_for(1, Dialect::Sqlite);
  assert_eq!(sqlite, "substr(?1, 1, ?2)");
  assert_eq!(sqlite_params.len(), 2);

  let (postgres, _) = raw.to_sql_fragment_for(5, Dialect::Postgres);
  assert_eq!(postgres, "substr($5, 1, $6)");
}
