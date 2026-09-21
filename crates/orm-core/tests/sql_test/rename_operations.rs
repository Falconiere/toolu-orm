//! `RenameTable` / `RenameColumn` render the same `ALTER TABLE … RENAME` in both dialects.

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::diff::Operation;
use toolu_orm_core::sql::generate_sql_for;

#[test]
fn rename_table_sql() {
  let ops = vec![Operation::RenameTable {
    old: "users".to_owned(),
    new: "accounts".to_owned(),
  }];
  let sql = generate_sql_for(&ops, Dialect::Postgres);
  assert_eq!(sql.trim(), "ALTER TABLE \"users\" RENAME TO \"accounts\";");
}

#[test]
fn rename_column_sql() {
  let ops = vec![Operation::RenameColumn {
    table: "users".to_owned(),
    old: "name".to_owned(),
    new: "full_name".to_owned(),
  }];
  let sql = generate_sql_for(&ops, Dialect::Postgres);
  assert_eq!(
    sql.trim(),
    "ALTER TABLE \"users\" RENAME COLUMN \"name\" TO \"full_name\";"
  );
}
