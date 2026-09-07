use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::diff::Operation;
use toolu_orm_core::sql::generate_sql_for;

#[test]
fn create_enum_postgres() {
  let ops = vec![Operation::CreateEnum {
    name: "status".to_owned(),
    variants: vec![
      "draft".to_owned(),
      "active".to_owned(),
      "archived".to_owned(),
    ],
  }];
  let sql = generate_sql_for(&ops, Dialect::Postgres);
  assert_eq!(
    sql.trim(),
    "CREATE TYPE \"status\" AS ENUM ('draft', 'active', 'archived');"
  );
}

#[test]
fn create_enum_sqlite_is_noop() {
  let ops = vec![Operation::CreateEnum {
    name: "status".to_owned(),
    variants: vec!["draft".to_owned()],
  }];
  let sql = generate_sql_for(&ops, Dialect::Sqlite);
  assert!(
    sql.trim().is_empty() || sql.contains("-- enum"),
    "sql: {sql}"
  );
}

#[test]
fn alter_enum_add_variant_postgres() {
  let ops = vec![Operation::AlterEnum {
    name: "status".to_owned(),
    added: vec!["suspended".to_owned()],
    removed: vec![],
  }];
  let sql = generate_sql_for(&ops, Dialect::Postgres);
  assert!(
    sql.contains("ALTER TYPE \"status\" ADD VALUE 'suspended'"),
    "sql: {sql}"
  );
}

#[test]
fn drop_enum_postgres() {
  let ops = vec![Operation::DropEnum {
    name: "status".to_owned(),
  }];
  let sql = generate_sql_for(&ops, Dialect::Postgres);
  assert_eq!(sql.trim(), "DROP TYPE IF EXISTS \"status\";");
}
