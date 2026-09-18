//! `ON` parameters are numbered before `WHERE` parameters, in every form.

use super::fixtures::ordered_builder;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::value::Value;

#[test]
fn on_params_precede_where_params_sqlite() {
  let (sql, params) = ordered_builder().to_sql_for(Dialect::Sqlite);
  assert_eq!(
    sql,
    concat!(
      r#"SELECT "code_symbols"."id" FROM "code_symbols" LEFT JOIN "code_feedback" ON "#,
      r#"("code_feedback"."repo" = "code_symbols"."repo" AND "code_feedback"."live" = ?1)"#,
      r#" WHERE "code_symbols"."kind" = ?2 LIMIT ?3"#
    )
  );
  assert_eq!(
    params,
    vec![
      Value::Integer(1),
      Value::Text("note".to_owned()),
      Value::Integer(5),
    ]
  );
}

#[test]
fn on_params_precede_where_params_postgres() {
  let (sql, params) = ordered_builder().to_sql_for(Dialect::Postgres);
  assert_eq!(
    sql,
    concat!(
      r#"SELECT "code_symbols"."id" FROM "code_symbols" LEFT JOIN "code_feedback" ON "#,
      r#"("code_feedback"."repo" = "code_symbols"."repo" AND "code_feedback"."live" = $1)"#,
      r#" WHERE "code_symbols"."kind" = $2 LIMIT $3"#
    )
  );
  assert_eq!(
    params,
    vec![
      Value::Integer(1),
      Value::Text("note".to_owned()),
      Value::Integer(5),
    ]
  );
}

#[test]
fn count_exists_and_first_row_thread_join_params() {
  let (count_sql, count_params) = ordered_builder().to_count_sql_for(Dialect::Sqlite);
  assert_eq!(
    count_sql,
    concat!(
      r#"SELECT COUNT(*) FROM "code_symbols" LEFT JOIN "code_feedback" ON "#,
      r#"("code_feedback"."repo" = "code_symbols"."repo" AND "code_feedback"."live" = ?1)"#,
      r#" WHERE "code_symbols"."kind" = ?2"#
    )
  );
  assert_eq!(
    count_params,
    vec![Value::Integer(1), Value::Text("note".to_owned())]
  );

  let (exists_sql, exists_params) = ordered_builder().to_exists_sql_for(Dialect::Postgres);
  assert_eq!(
    exists_sql,
    concat!(
      r#"SELECT EXISTS(SELECT 1 FROM "code_symbols" LEFT JOIN "code_feedback" ON "#,
      r#"("code_feedback"."repo" = "code_symbols"."repo" AND "code_feedback"."live" = $1)"#,
      r#" WHERE "code_symbols"."kind" = $2)"#
    )
  );
  assert_eq!(exists_params, count_params);

  let (first_sql, first_params) = ordered_builder().to_first_row_sql_for(Dialect::Sqlite);
  assert!(first_sql.contains(r#""code_feedback"."live" = ?1"#));
  assert!(first_sql.ends_with(r#"WHERE "code_symbols"."kind" = ?2 LIMIT ?3"#));
  assert_eq!(
    first_params,
    vec![
      Value::Integer(1),
      Value::Text("note".to_owned()),
      Value::Integer(1),
    ]
  );
}
