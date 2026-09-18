//! One ordered parameter list per statement: `VALUES` first, then the
//! conflict clause, whatever expressions either side carries.

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::expr::Scalar;
use toolu_orm_core::value::Value;
use toolu_orm_query::insert::{InsertBuilder, OnConflict};

use super::columns::{INDEXED_AT, ISO_FORMAT, MEMORY_ID, PATH, REPO, SYMBOL_ID, USED_COUNT};

type TestResult = Result<(), DbCoreError>;

/// `strftime('%Y-%m-%dT%H:%M:%fZ', 'now')` with both arguments bound.
fn database_clock() -> Result<Scalar, DbCoreError> {
  Scalar::func(
    "strftime",
    vec![Scalar::bind(ISO_FORMAT), Scalar::bind("now")],
  )
}

fn code_symbol_upsert() -> Result<InsertBuilder, DbCoreError> {
  Ok(
    InsertBuilder::new("code_symbols")
      .set(&REPO, "toolu-orm")
      .set(&PATH, "src/lib.rs")
      .set_scalar(&INDEXED_AT, database_clock()?)
      .on_conflict(
        OnConflict::column(&REPO)
          .and_column(&PATH)
          .set_scalar(&INDEXED_AT, database_clock()?),
      )
      .returning(&SYMBOL_ID),
  )
}

#[test]
fn expression_values_then_expression_assignments_number_left_to_right() -> TestResult {
  let (sql, params) = code_symbol_upsert()?.to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    concat!(
      r#"INSERT INTO "code_symbols" ("repo", "path", "indexed_at") "#,
      r#"VALUES (?1, ?2, strftime(?3, ?4)) "#,
      r#"ON CONFLICT ("repo", "path") DO UPDATE SET "indexed_at" = strftime(?5, ?6) "#,
      r#"RETURNING "id""#
    )
  );
  assert_eq!(
    params,
    vec![
      Value::Text("toolu-orm".to_owned()),
      Value::Text("src/lib.rs".to_owned()),
      Value::Text(ISO_FORMAT.to_owned()),
      Value::Text("now".to_owned()),
      Value::Text(ISO_FORMAT.to_owned()),
      Value::Text("now".to_owned()),
    ]
  );
  Ok(())
}

#[test]
fn the_same_statement_numbers_dollar_placeholders_in_the_same_order() -> TestResult {
  let (sql, params) = code_symbol_upsert()?.to_sql_for(Dialect::Postgres);

  assert_eq!(
    sql,
    concat!(
      r#"INSERT INTO "code_symbols" ("repo", "path", "indexed_at") "#,
      r#"VALUES ($1, $2, strftime($3, $4)) "#,
      r#"ON CONFLICT ("repo", "path") DO UPDATE SET "indexed_at" = strftime($5, $6) "#,
      r#"RETURNING "id""#
    )
  );
  assert_eq!(
    params,
    vec![
      Value::Text("toolu-orm".to_owned()),
      Value::Text("src/lib.rs".to_owned()),
      Value::Text(ISO_FORMAT.to_owned()),
      Value::Text("now".to_owned()),
      Value::Text(ISO_FORMAT.to_owned()),
      Value::Text("now".to_owned()),
    ],
    "the Postgres renderer must emit the same values in the same order"
  );
  Ok(())
}

#[test]
fn a_clause_numbers_from_one_when_the_values_bind_nothing() {
  let (sql, params) = InsertBuilder::new("code_symbols")
    .set_scalar(&REPO, Scalar::sql("'literal'"))
    .on_conflict(OnConflict::column(&REPO).set(&PATH, "src/main.rs"))
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    concat!(
      r#"INSERT INTO "code_symbols" ("repo") VALUES ('literal') "#,
      r#"ON CONFLICT ("repo") DO UPDATE SET "path" = ?1"#
    )
  );
  assert_eq!(params, vec![Value::Text("src/main.rs".to_owned())]);
}

#[test]
fn a_raw_assignment_renumbers_its_own_placeholders_after_the_values() {
  let (sql, params) = InsertBuilder::new("memories")
    .set(&MEMORY_ID, "m1")
    .on_conflict(OnConflict::column(&MEMORY_ID).set_scalar(
      &USED_COUNT,
      Scalar::raw(
        r#"max("memories"."used_count", ?)"#,
        vec![Value::Integer(7)],
      ),
    ))
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    concat!(
      r#"INSERT INTO "memories" ("id") VALUES (?1) "#,
      r#"ON CONFLICT ("id") DO UPDATE SET "used_count" = max("memories"."used_count", ?2)"#
    )
  );
  assert_eq!(
    params,
    vec![Value::Text("m1".to_owned()), Value::Integer(7)]
  );
}
