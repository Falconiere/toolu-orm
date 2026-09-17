use toolu_orm_core::column::ColumnType;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::diff::{ColumnChange, Operation};
use toolu_orm_core::index::{IndexColumn, IndexDef};
use toolu_orm_core::sql::generate_sql_for;
use toolu_orm_core::table::{TableDef, TableKind};

use super::column_helpers::{col, col_with_default};

/// `users(id PK, name NOT NULL)` with a unique index on `name`.
fn users_table() -> TableDef {
  TableDef {
    name: "users".to_owned(),
    columns: vec![
      col("id", ColumnType::Text, true, false),
      col("name", ColumnType::Text, false, true),
    ],
    indexes: vec![IndexDef {
      name: "idx_users_name".to_owned(),
      columns: vec![IndexColumn::new("name")],
      unique: true,
      where_clause: None,
    }],
    primary_key: vec![],
    strict: false,
    kind: TableKind::Ordinary,
    fts5_sync: None,
  }
}

fn name_not_null() -> ColumnChange {
  ColumnChange::Nullable {
    column: "name".to_owned(),
    old: true,
    new: false,
  }
}

fn alter_users(table: TableDef) -> Operation {
  Operation::AlterColumn {
    table: table.name.clone(),
    changes: vec![name_not_null()],
    table_def: table,
  }
}

/// Byte offset of `needle`, failing with the whole statement list when absent.
fn offset(sql: &str, needle: &str) -> usize {
  assert!(sql.contains(needle), "missing `{needle}` in:\n{sql}");
  sql.find(needle).unwrap_or_default()
}

#[test]
fn test_add_column_sql() {
  let ops = vec![Operation::AddColumn {
    table: "conversations".to_owned(),
    column: col("title", ColumnType::Text, false, false),
  }];
  let sql = generate_sql_for(&ops, Dialect::Sqlite);
  assert!(sql.contains("ALTER TABLE \"conversations\" ADD COLUMN \"title\" TEXT"));
}

#[test]
fn test_drop_column_sqlite_uses_alter_drop() {
  let ops = vec![Operation::DropColumn {
    table: "users".to_owned(),
    column: "legacy_field".to_owned(),
  }];
  let sql = generate_sql_for(&ops, Dialect::Sqlite);
  assert!(sql.contains(r#"DROP COLUMN "legacy_field""#), "sql: {sql}");
}

#[test]
fn alter_column_rebuilds_with_create_copy_drop_rename() {
  let sql = generate_sql_for(&[alter_users(users_table())], Dialect::Sqlite);

  let create = offset(&sql, r#"CREATE TABLE "_toolu_new_users""#);
  let copy = offset(
    &sql,
    r#"INSERT INTO "_toolu_new_users" ("id", "name") SELECT "id", "name" FROM "users";"#,
  );
  let drop = offset(&sql, r#"DROP TABLE "users";"#);
  let rename = offset(&sql, r#"ALTER TABLE "_toolu_new_users" RENAME TO "users";"#);

  assert!(
    create < copy && copy < drop && drop < rename,
    "wrong rebuild order: {sql}"
  );
  assert!(
    !sql.contains("_users_old"),
    "the old table must never be renamed out of the way: {sql}"
  );
  assert!(
    !sql.contains(r#"CREATE TABLE IF NOT EXISTS "_toolu_new_users""#),
    "a taken staging name must fail, not be adopted: {sql}"
  );
  assert!(sql.contains("PRAGMA foreign_keys = OFF"), "sql: {sql}");
  assert!(sql.contains("NOT NULL"), "sql: {sql}");

  // The rename has to run in legacy mode: otherwise SQLite re-parses every
  // view and trigger and rejects any that still names the dropped table.
  let legacy_on = offset(&sql, "PRAGMA legacy_alter_table = ON;");
  let legacy_off = offset(&sql, "PRAGMA legacy_alter_table = OFF;");
  assert!(
    legacy_on < rename && rename < legacy_off,
    "the rename is not wrapped in legacy_alter_table: {sql}"
  );
}

#[test]
fn rebuild_recreates_every_declared_index() {
  let sql = generate_sql_for(&[alter_users(users_table())], Dialect::Sqlite);
  let index = offset(
    &sql,
    r#"CREATE UNIQUE INDEX IF NOT EXISTS "idx_users_name" ON "users" ("name")"#,
  );
  let rename = offset(&sql, r#"RENAME TO "users";"#);
  assert!(rename < index, "index created before the rename: {sql}");
}

#[test]
fn rebuild_absorbs_an_added_column_and_does_not_copy_it() {
  let mut table = users_table();
  table
    .columns
    .push(col_with_default("age", ColumnType::Integer, false, "0"));
  let ops = vec![
    alter_users(table.clone()),
    Operation::AddColumn {
      table: "users".to_owned(),
      column: col_with_default("age", ColumnType::Integer, false, "0"),
    },
  ];
  let sql = generate_sql_for(&ops, Dialect::Sqlite);

  assert!(
    sql.contains(
      r#"INSERT INTO "_toolu_new_users" ("id", "name") SELECT "id", "name" FROM "users";"#
    ),
    "the added column must not be selected from the old table: {sql}"
  );
  assert!(
    !sql.contains("ADD COLUMN"),
    "the rebuild already carries the new column: {sql}"
  );
  assert!(
    sql.contains(r#""age" INTEGER DEFAULT (0)"#),
    "the staging table must declare the added column: {sql}"
  );
}

#[test]
fn rebuild_absorbs_a_dropped_column() {
  let ops = vec![
    alter_users(users_table()),
    Operation::DropColumn {
      table: "users".to_owned(),
      column: "bio".to_owned(),
    },
  ];
  let sql = generate_sql_for(&ops, Dialect::Sqlite);
  assert!(
    !sql.contains("DROP COLUMN"),
    "the rebuild already leaves the column out: {sql}"
  );
  assert!(!sql.contains(r#""bio""#), "sql: {sql}");
}

#[test]
fn each_altered_table_is_rebuilt_exactly_once() {
  let mut posts = users_table();
  posts.name = "posts".to_owned();
  posts.indexes = vec![];
  let ops = vec![
    alter_users(users_table()),
    Operation::AlterColumn {
      table: "users".to_owned(),
      changes: vec![ColumnChange::Type {
        column: "id".to_owned(),
        old: ColumnType::Integer,
        new: ColumnType::Text,
      }],
      table_def: users_table(),
    },
    alter_users(posts),
  ];
  let sql = generate_sql_for(&ops, Dialect::Sqlite);
  assert_eq!(
    sql.matches(r#"CREATE TABLE "_toolu_new_users""#).count(),
    1,
    "users rebuilt more than once: {sql}"
  );
  assert_eq!(
    sql.matches(r#"CREATE TABLE "_toolu_new_posts""#).count(),
    1,
    "posts rebuilt more than once: {sql}"
  );
}

#[test]
fn postgres_alters_in_place_and_never_rebuilds() {
  let mut table = users_table();
  table
    .columns
    .push(col_with_default("age", ColumnType::Integer, false, "0"));
  let ops = vec![
    alter_users(table),
    Operation::AddColumn {
      table: "users".to_owned(),
      column: col_with_default("age", ColumnType::Integer, false, "0"),
    },
  ];
  let sql = generate_sql_for(&ops, Dialect::Postgres);
  assert!(
    sql.contains(r#"ALTER TABLE "users" ALTER COLUMN "name" SET NOT NULL;"#),
    "sql: {sql}"
  );
  assert!(sql.contains(r#"ADD COLUMN "age""#), "sql: {sql}");
  assert!(!sql.contains("_toolu_new_"), "sql: {sql}");
  assert!(!sql.contains("DROP TABLE"), "sql: {sql}");
}
