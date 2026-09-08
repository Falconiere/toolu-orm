//! `SelectBuilder` carrying an FTS5 projection: the whole query from issue #20
//! built through the builder, and the select-list rules that make it possible.

use toolu_orm_core::column::Text;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::{Expr, OrderBy};
use toolu_orm_core::fts5;
use toolu_orm_core::query_column::{Column, CommonOps};
use toolu_orm_core::value::Value;
use toolu_orm_query::select::SelectBuilder;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const FTS_MEMORY_ID: Column<Text> = Column::new("memory_fts", "memory_id");
const MEMORY_ID: Column<Text> = Column::new("memories", "id");
const MEMORY_DELETED_AT: Column<Text> = Column::new("memories", "deleted_at");

/// The query issue #20 opens with, previously impossible to express: a `bm25`
/// projection, a join, a table-level `MATCH` sharing one parameter sequence
/// with an ordinary filter, and `ORDER BY` the computed alias.
#[test]
fn the_full_text_query_from_the_issue_is_expressible() -> TestResult {
  let score = fts5::bm25_for(Dialect::Sqlite, "memory_fts", &[0.0, 1.0, 3.0])?;

  let (sql, params) = SelectBuilder::new("memory_fts")
    .columns_raw(&["memory_id"])
    .column_expr(score.sql(), "score")
    .join("memories", MEMORY_ID.equals(&FTS_MEMORY_ID))
    .filter(Expr::table_match_for(
      Dialect::Sqlite,
      "memory_fts",
      "runner",
    )?)
    .filter(MEMORY_DELETED_AT.is_null())
    .order_by(OrderBy::alias_asc("score"))
    .limit(10)
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    concat!(
      r#"SELECT "memory_id", bm25("memory_fts", 0.0, 1.0, 3.0) AS "score""#,
      r#" FROM "memory_fts""#,
      r#" INNER JOIN "memories" ON "memories"."id" = "memory_fts"."memory_id""#,
      r#" WHERE "memory_fts" MATCH ?1 AND "memories"."deleted_at" IS NULL"#,
      r#" ORDER BY "score" ASC LIMIT ?2"#
    )
  );
  assert_eq!(
    params,
    vec![Value::Text("runner".to_owned()), Value::Integer(10)]
  );
  Ok(())
}

/// Ordering by the call itself rather than by an alias, for a query that does
/// not project the score.
#[test]
fn a_query_can_order_by_the_score_without_selecting_it() -> TestResult {
  let (sql, _) = SelectBuilder::new("memory_fts")
    .columns_raw(&["memory_id"])
    .filter(Expr::table_match_for(
      Dialect::Sqlite,
      "memory_fts",
      "runner",
    )?)
    .order_by(fts5::rank_for(Dialect::Sqlite, "memory_fts")?.asc())
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"SELECT "memory_id" FROM "memory_fts" WHERE "memory_fts" MATCH ?1 ORDER BY rank ASC"#
  );
  Ok(())
}

/// The behavior this change fixes: a non-raw builder used to drop its
/// `column_expr` entirely.
#[test]
fn a_non_raw_builder_keeps_a_column_expr_alongside_its_columns() -> TestResult {
  let score = fts5::bm25_for(Dialect::Sqlite, "memory_fts", &[1.0])?;
  let (sql, _) = SelectBuilder::new("memory_fts")
    .columns_raw(&["memory_id", "body"])
    .column_expr(score.sql(), "score")
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"SELECT "memory_id", "body", bm25("memory_fts", 1.0) AS "score" FROM "memory_fts""#
  );
  Ok(())
}

#[test]
fn a_column_expr_without_columns_renders_no_leading_comma() -> TestResult {
  let score = fts5::bm25_for(Dialect::Sqlite, "memory_fts", &[1.0])?;
  let (sql, _) = SelectBuilder::new("memory_fts")
    .column_expr(score.sql(), "score")
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"SELECT bm25("memory_fts", 1.0) AS "score" FROM "memory_fts""#
  );
  Ok(())
}

/// `count` and `exists` render their own select list, so a projection never
/// leaks into them — only the `MATCH` filter does.
#[test]
fn count_and_exists_ignore_the_projection_but_keep_the_match() -> TestResult {
  let score = fts5::bm25_for(Dialect::Sqlite, "memory_fts", &[1.0])?;
  let builder = SelectBuilder::new("memory_fts")
    .columns_raw(&["memory_id"])
    .column_expr(score.sql(), "score")
    .filter(Expr::table_match_for(
      Dialect::Sqlite,
      "memory_fts",
      "runner",
    )?);

  let (count_sql, count_params) = builder.to_count_sql_for(Dialect::Sqlite);
  assert_eq!(
    count_sql,
    r#"SELECT COUNT(*) FROM "memory_fts" WHERE "memory_fts" MATCH ?1"#
  );
  assert_eq!(count_params, vec![Value::Text("runner".to_owned())]);

  let (exists_sql, _) = builder.to_exists_sql_for(Dialect::Sqlite);
  assert_eq!(
    exists_sql,
    r#"SELECT EXISTS(SELECT 1 FROM "memory_fts" WHERE "memory_fts" MATCH ?1)"#
  );
  Ok(())
}
