//! `Scalar::func`: rendering, nested binds, and name validation.

use toolu_orm_core::column::Text;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::expr::Scalar;
use toolu_orm_core::query_column::Column;
use toolu_orm_core::value::Value;

const CREATED_AT: Column<Text> = Column::new("memories", "created_at");
const LAST_SEEN: Column<Text> = Column::new("memories", "last_accessed");

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn a_call_over_a_column_renders_on_both_dialects() -> TestResult {
  let call = Scalar::func("datetime", vec![Scalar::col(&CREATED_AT)])?;

  assert_eq!(
    call.to_sql_fragment_for(1, Dialect::Sqlite).0,
    r#"datetime("memories"."created_at")"#
  );
  assert_eq!(
    call.to_sql_fragment_for(1, Dialect::Postgres).0,
    r#"datetime("memories"."created_at")"#
  );
  Ok(())
}

#[test]
fn nested_arguments_number_left_to_right_from_the_offset() -> TestResult {
  let call = Scalar::func(
    "coalesce",
    vec![
      Scalar::bind("a"),
      Scalar::func("substr", vec![Scalar::col(&LAST_SEEN), Scalar::bind(1)])?,
      Scalar::bind("z"),
    ],
  )?;

  let (sql, params) = call.to_sql_fragment_for(3, Dialect::Sqlite);
  assert_eq!(
    sql,
    r#"coalesce(?3, substr("memories"."last_accessed", ?4), ?5)"#
  );
  assert_eq!(
    params,
    vec![
      Value::Text("a".to_owned()),
      Value::Integer(1),
      Value::Text("z".to_owned())
    ]
  );
  Ok(())
}

#[test]
fn a_call_without_arguments_renders_empty_parentheses() -> TestResult {
  assert_eq!(
    Scalar::func("unixepoch", Vec::new())?
      .to_sql_fragment_for(1, Dialect::Sqlite)
      .0,
    "unixepoch()"
  );
  Ok(())
}

#[test]
fn an_underscore_leading_name_is_accepted() -> TestResult {
  assert_eq!(
    Scalar::func("_x9", Vec::new())?
      .to_sql_fragment_for(1, Dialect::Sqlite)
      .0,
    "_x9()"
  );
  Ok(())
}

#[test]
fn a_name_that_is_not_a_plain_identifier_is_refused() -> TestResult {
  for name in ["", "drop table users; --", "a b", "1abc", "fn(", "ré"] {
    let Err(DbCoreError::InvalidScalarFunction { name: reported }) = Scalar::func(name, Vec::new())
    else {
      return Err(format!("expected InvalidScalarFunction for {name:?}").into());
    };
    assert_eq!(reported, name);
  }
  Ok(())
}
