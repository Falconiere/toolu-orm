//! The issue #40 query through `SelectBuilder`: `@@`, `ts_rank`, soft-delete,
//! and `ORDER BY` alias — pure SQL, no database.

use toolu_orm_core::column::{Integer, Text};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::OrderBy;
use toolu_orm_core::pg_fts;
use toolu_orm_core::query_column::{Column, CommonOps};
use toolu_orm_core::value::Value;
use toolu_orm_query::select::SelectBuilder;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const SEARCH: Column<Text> = Column::new("docs", "search_vector");
const DELETED_AT: Column<Integer> = Column::new("docs", "deleted_at");

#[test]
fn the_full_text_query_from_the_issue_is_expressible() -> TestResult {
  let doc = pg_fts::column_for(Dialect::Postgres, &SEARCH)?;
  let score = pg_fts::ts_rank_tsquery_for(Dialect::Postgres, &doc, "english", "runner", None)?;

  let (sql, params) = SelectBuilder::new("docs")
    .columns_raw(&["id"])
    .column_expr(score.sql(), "score")
    .filter(doc.matches_tsquery_for(Dialect::Postgres, "english", "runner")?)
    .filter(DELETED_AT.is_null())
    .order_by(OrderBy::alias_desc("score"))
    .limit(10)
    .to_sql_for(Dialect::Postgres);

  assert_eq!(
    sql,
    concat!(
      r#"SELECT "id", "#,
      r#"ts_rank("docs"."search_vector", to_tsquery('english', E'runner')) AS "score""#,
      r#" FROM "docs""#,
      r#" WHERE "docs"."search_vector" @@ to_tsquery('english', $1)"#,
      r#" AND "docs"."deleted_at" IS NULL"#,
      r#" ORDER BY "score" DESC LIMIT $2"#,
    )
  );
  assert_eq!(
    params,
    vec![Value::Text("runner".to_owned()), Value::Integer(10)]
  );
  assert!(!sql.contains("MATCH"));
  assert!(!sql.contains("bm25"));
  Ok(())
}

#[test]
fn a_query_can_order_by_ts_rank_without_selecting_it() -> TestResult {
  let doc = pg_fts::column_for(Dialect::Postgres, &SEARCH)?;
  let score = pg_fts::ts_rank_tsquery_for(Dialect::Postgres, &doc, "english", "runner", None)?;

  let (sql, _) = SelectBuilder::new("docs")
    .columns_raw(&["id"])
    .filter(doc.matches_tsquery_for(Dialect::Postgres, "english", "runner")?)
    .order_by(score.desc())
    .to_sql_for(Dialect::Postgres);

  assert_eq!(
    sql,
    concat!(
      r#"SELECT "id" FROM "docs""#,
      r#" WHERE "docs"."search_vector" @@ to_tsquery('english', $1)"#,
      r#" ORDER BY ts_rank("docs"."search_vector", to_tsquery('english', E'runner')) DESC"#,
    )
  );
  Ok(())
}
