//! `.offset(n)` without a paired `.limit(...)`: SQLite rejects a standalone
//! `OFFSET`, so it must render `LIMIT -1 OFFSET ...` instead. Rendered without
//! a database. See issue #92.

use toolu_orm_core::column::{Integer, Text};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::query_column::{Column, CommonOps};
use toolu_orm_core::value::Value;
use toolu_orm_query::select::SelectBuilder;

const ORG_ID: Column<Text> = Column::new("users", "org_id");
const AGE: Column<Integer> = Column::new("users", "age");

#[test]
fn sqlite_offset_only_renders_limit_negative_one() {
  let (sql, params) = SelectBuilder::new("users")
    .columns_raw(&["id"])
    .offset(7)
    .to_sql_for(Dialect::Sqlite);
  assert_eq!(sql, r#"SELECT "id" FROM "users" LIMIT -1 OFFSET ?1"#);
  assert_eq!(params, vec![Value::Integer(7)]);
}

#[test]
fn sqlite_offset_zero_still_renders_limit_negative_one() {
  let (sql, params) = SelectBuilder::new("users")
    .columns_raw(&["id"])
    .offset(0)
    .to_sql_for(Dialect::Sqlite);
  assert_eq!(sql, r#"SELECT "id" FROM "users" LIMIT -1 OFFSET ?1"#);
  assert_eq!(params, vec![Value::Integer(0)]);
}

/// Postgres accepts a standalone `OFFSET`; the fix must not touch this path.
#[test]
fn postgres_offset_only_keeps_standalone_offset() {
  let (sql, params) = SelectBuilder::new("users")
    .columns_raw(&["id"])
    .offset(7)
    .to_sql_for(Dialect::Postgres);
  assert_eq!(sql, r#"SELECT "id" FROM "users" OFFSET $1"#);
  assert_eq!(params, vec![Value::Integer(7)]);
}

/// An explicit `.limit(...)` alongside `.offset(...)` is the pre-existing,
/// already-correct path: no literal `LIMIT -1` involved.
#[test]
fn sqlite_explicit_limit_and_offset_unaffected() {
  let (sql, params) = SelectBuilder::new("users")
    .columns_raw(&["id"])
    .limit(5)
    .offset(2)
    .to_sql_for(Dialect::Sqlite);
  assert_eq!(sql, r#"SELECT "id" FROM "users" LIMIT ?1 OFFSET ?2"#);
  assert_eq!(params, vec![Value::Integer(5), Value::Integer(2)]);
}

/// The literal `LIMIT -1` contributes no parameter, so a WHERE filter ahead
/// of offset-only pagination must still number `OFFSET` right after the
/// filter's own placeholders.
#[test]
fn sqlite_where_filter_then_offset_only_keeps_parameter_numbering() {
  let (sql, params) = SelectBuilder::new("users")
    .columns_raw(&["id"])
    .filter(ORG_ID.eq("org123"))
    .offset(9)
    .to_sql_for(Dialect::Sqlite);
  assert_eq!(
    sql,
    r#"SELECT "id" FROM "users" WHERE "users"."org_id" = ?1 LIMIT -1 OFFSET ?2"#
  );
  assert_eq!(
    params,
    vec![Value::Text("org123".to_owned()), Value::Integer(9)]
  );
}

/// Postgres twin of the previous case: no literal `LIMIT`, so `OFFSET`
/// numbers right after the WHERE filter's own placeholder.
#[test]
fn postgres_where_filter_then_offset_only_keeps_parameter_numbering() {
  let (sql, params) = SelectBuilder::new("users")
    .columns_raw(&["id"])
    .filter(AGE.eq(30))
    .offset(9)
    .to_sql_for(Dialect::Postgres);
  assert_eq!(
    sql,
    r#"SELECT "id" FROM "users" WHERE "users"."age" = $1 OFFSET $2"#
  );
  assert_eq!(params, vec![Value::Integer(30), Value::Integer(9)]);
}
