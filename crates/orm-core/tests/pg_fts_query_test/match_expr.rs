//! `@@` as an [`Expr`]: column and `to_tsvector` documents, query-fn variants,
//! and parameter numbering.

use toolu_orm_core::column::Text;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::pg_fts;
use toolu_orm_core::query_column::{Column, CommonOps};
use toolu_orm_core::value::Value;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const SEARCH: Column<Text> = Column::new("docs", "search_vector");
const BODY: Column<Text> = Column::new("docs", "body");

#[test]
fn column_tsquery_renders_and_binds_the_pattern() -> TestResult {
  let doc = pg_fts::column_for(Dialect::Postgres, &SEARCH)?;
  let expr = doc.matches_tsquery_for(Dialect::Postgres, "english", "runner")?;
  let (sql, params) = expr.to_sql_fragment_for(1, Dialect::Postgres);

  assert_eq!(
    sql,
    r#""docs"."search_vector" @@ to_tsquery('english', $1)"#
  );
  assert_eq!(params, vec![Value::Text("runner".to_owned())]);
  Ok(())
}

#[test]
fn tsquery_continues_the_callers_parameter_numbering() -> TestResult {
  let doc = pg_fts::column_for(Dialect::Postgres, &SEARCH)?;
  let expr = doc.matches_tsquery_for(Dialect::Postgres, "english", "runner")?;
  let (sql, params) = expr.to_sql_fragment_for(3, Dialect::Postgres);

  assert_eq!(
    sql,
    r#""docs"."search_vector" @@ to_tsquery('english', $3)"#
  );
  assert_eq!(params.len(), 1);
  Ok(())
}

#[test]
fn to_tsvector_plainto_and_websearch_render() -> TestResult {
  let doc = pg_fts::to_tsvector_for(Dialect::Postgres, "english", &BODY)?;

  let plain = doc.matches_plainto_tsquery_for(Dialect::Postgres, "english", "marathon runner")?;
  let (sql, params) = plain.to_sql_fragment_for(1, Dialect::Postgres);
  assert_eq!(
    sql,
    r#"to_tsvector('english', "docs"."body") @@ plainto_tsquery('english', $1)"#
  );
  assert_eq!(params, vec![Value::Text("marathon runner".to_owned())]);

  let web = doc.matches_websearch_to_tsquery_for(Dialect::Postgres, "english", "runner -shoes")?;
  let (sql, params) = web.to_sql_fragment_for(1, Dialect::Postgres);
  assert_eq!(
    sql,
    r#"to_tsvector('english', "docs"."body") @@ websearch_to_tsquery('english', $1)"#
  );
  assert_eq!(params, vec![Value::Text("runner -shoes".to_owned())]);
  Ok(())
}

#[test]
fn match_composes_with_ordinary_filters() -> TestResult {
  let deleted: Column<Text> = Column::new("docs", "deleted_at");
  let doc = pg_fts::column_for(Dialect::Postgres, &SEARCH)?;
  let expr = doc
    .matches_tsquery_for(Dialect::Postgres, "english", "runner")?
    .and(deleted.is_null());
  let (sql, params) = expr.to_sql_fragment_for(1, Dialect::Postgres);

  assert_eq!(
    sql,
    r#"("docs"."search_vector" @@ to_tsquery('english', $1) AND "docs"."deleted_at" IS NULL)"#
  );
  assert_eq!(params, vec![Value::Text("runner".to_owned())]);
  Ok(())
}

#[test]
fn the_short_forms_agree_with_the_current_dialect() {
  let short = pg_fts::column(&SEARCH);
  let explicit = pg_fts::column_for(Dialect::CURRENT, &SEARCH);
  assert_eq!(short.is_ok(), explicit.is_ok());

  if let (Ok(short_doc), Ok(explicit_doc)) = (short, explicit) {
    let short_m = short_doc.matches_tsquery("english", "runner");
    let explicit_m = explicit_doc.matches_tsquery_for(Dialect::CURRENT, "english", "runner");
    assert_eq!(short_m.is_ok(), explicit_m.is_ok());
  }
}
