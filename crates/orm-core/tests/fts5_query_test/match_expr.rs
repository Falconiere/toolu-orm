//! `MATCH` as an `Expr`: the whole-table form, the column-qualified form, and
//! their place in a caller's parameter sequence.

use toolu_orm_core::column::Text;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::Expr;
use toolu_orm_core::query_column::{Column, CommonOps, Fts5Ops};
use toolu_orm_core::value::Value;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const BODY: Column<Text> = Column::new("memory_fts", "body");

#[test]
fn table_match_renders_the_quoted_table_and_binds_the_pattern() -> TestResult {
  let expr = Expr::table_match_for(Dialect::Sqlite, "memory_fts", "runner")?;
  let (sql, params) = expr.to_sql_fragment_for(1, Dialect::Sqlite);

  assert_eq!(sql, r#""memory_fts" MATCH ?1"#);
  assert_eq!(params, vec![Value::Text("runner".to_owned())]);
  Ok(())
}

#[test]
fn table_match_continues_the_callers_parameter_numbering() -> TestResult {
  let expr = Expr::table_match_for(Dialect::Sqlite, "memory_fts", "runner")?;
  let (sql, params) = expr.to_sql_fragment_for(3, Dialect::Sqlite);

  assert_eq!(sql, r#""memory_fts" MATCH ?3"#);
  assert_eq!(params.len(), 1);
  Ok(())
}

#[test]
fn column_match_qualifies_the_column() -> TestResult {
  let expr = BODY.matches_for(Dialect::Sqlite, "zebrafish")?;
  let (sql, params) = expr.to_sql_fragment_for(1, Dialect::Sqlite);

  assert_eq!(sql, r#""memory_fts"."body" MATCH ?1"#);
  assert_eq!(params, vec![Value::Text("zebrafish".to_owned())]);
  Ok(())
}

#[test]
fn match_composes_with_the_ordinary_operators_and_shares_their_numbering() -> TestResult {
  let deleted_at: Column<Text> = Column::new("memories", "deleted_at");
  let expr = Expr::table_match_for(Dialect::Sqlite, "memory_fts", "runner")?
    .and(deleted_at.is_null())
    .and(BODY.matches_for(Dialect::Sqlite, "shoes")?);
  let (sql, params) = expr.to_sql_fragment_for(1, Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"(("memory_fts" MATCH ?1 AND "memories"."deleted_at" IS NULL) AND "memory_fts"."body" MATCH ?2)"#
  );
  assert_eq!(
    params,
    vec![
      Value::Text("runner".to_owned()),
      Value::Text("shoes".to_owned()),
    ]
  );
  Ok(())
}

/// The short forms follow `Dialect::CURRENT`, which is Sqlite on the default
/// lane and Postgres on the postgres lane. Agreement with the explicit call is
/// therefore the only assertion that holds on both.
#[test]
fn the_short_match_forms_agree_with_the_current_dialect() {
  let short = Expr::table_match("memory_fts", "runner");
  let explicit = Expr::table_match_for(Dialect::CURRENT, "memory_fts", "runner");
  assert_eq!(short.is_ok(), explicit.is_ok());

  let short_column = BODY.matches("runner");
  let explicit_column = BODY.matches_for(Dialect::CURRENT, "runner");
  assert_eq!(short_column.is_ok(), explicit_column.is_ok());
}
