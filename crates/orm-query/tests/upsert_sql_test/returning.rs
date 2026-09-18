//! `RETURNING` projections: unqualified, in call order, binding nothing, and
//! rendered after the conflict clause on both dialects.

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::value::Value;
use toolu_orm_query::insert::{InsertBuilder, OnConflict};

use super::columns::{INDEXED_AT, MEMORY_ID, PATH, REPO, SYMBOL_ID};

#[test]
fn returning_projects_its_columns_unqualified_in_call_order() {
  let (sql, params) = InsertBuilder::new("code_symbols")
    .set(&REPO, "r")
    .returning(&SYMBOL_ID)
    .returning(&PATH)
    .returning(&INDEXED_AT)
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"INSERT INTO "code_symbols" ("repo") VALUES (?1) RETURNING "id", "path", "indexed_at""#
  );
  assert_eq!(
    params,
    vec![Value::Text("r".to_owned())],
    "the one bind is the VALUES; the three projected columns add none"
  );
}

#[test]
fn returning_follows_the_conflict_clause_on_postgres() {
  let (sql, _) = InsertBuilder::new("memories")
    .set(&MEMORY_ID, "m1")
    .on_conflict(OnConflict::column(&MEMORY_ID))
    .returning(&MEMORY_ID)
    .to_sql_for(Dialect::Postgres);

  assert_eq!(
    sql,
    r#"INSERT INTO "memories" ("id") VALUES ($1) ON CONFLICT ("id") DO NOTHING RETURNING "id""#
  );
}

#[test]
fn returning_also_follows_the_legacy_or_ignore_shorthand() {
  let (sqlite, _) = InsertBuilder::new("memories")
    .set(&MEMORY_ID, "m1")
    .or_ignore()
    .returning(&MEMORY_ID)
    .to_sql_for(Dialect::Sqlite);
  assert_eq!(
    sqlite,
    r#"INSERT OR IGNORE INTO "memories" ("id") VALUES (?1) RETURNING "id""#
  );

  let (postgres, _) = InsertBuilder::new("memories")
    .set(&MEMORY_ID, "m1")
    .or_ignore()
    .returning(&MEMORY_ID)
    .to_sql_for(Dialect::Postgres);
  assert_eq!(
    postgres,
    r#"INSERT INTO "memories" ("id") VALUES ($1) ON CONFLICT DO NOTHING RETURNING "id""#
  );
}

#[test]
fn no_returning_call_leaves_the_statement_unchanged() {
  let (sql, _) = InsertBuilder::new("memories")
    .set(&MEMORY_ID, "m1")
    .to_sql_for(Dialect::Sqlite);
  assert_eq!(sql, r#"INSERT INTO "memories" ("id") VALUES (?1)"#);
}
