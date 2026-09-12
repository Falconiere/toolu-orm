//! Key-set changes force SQLite table recreation.

use toolu_orm_core::column::ColumnType;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::diff::{diff_with_resolver, ColumnChange, Operation};
use toolu_orm_core::rename::NoRenames;
use toolu_orm_core::schema::SchemaRegistry;
use toolu_orm_core::snapshot::Snapshot;
use toolu_orm_core::sql::generate_sql_for;
use toolu_orm_core::table::TableDef;

use super::{col, table};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn composite_primary_key_change_recreates_table() -> TestResult {
  let old_table = TableDef {
    name: "memory_tags".to_owned(),
    columns: vec![
      col("memory_id", ColumnType::Text, false, true),
      col("tag", ColumnType::Text, false, true),
    ],
    indexes: vec![],
    primary_key: vec!["memory_id".to_owned()],
    strict: false,
    kind: toolu_orm_core::table::TableKind::Ordinary,
  };
  let new_table = TableDef {
    primary_key: vec!["memory_id".to_owned(), "tag".to_owned()],
    ..old_table.clone()
  };
  let old = Snapshot::from_registry(&SchemaRegistry::from_tables(vec![old_table]));
  let new_reg = SchemaRegistry::from_tables(vec![new_table]);
  let ops = diff_with_resolver(&old, &new_reg, &NoRenames)?;
  assert_eq!(ops.len(), 1);
  let op = ops.first().ok_or("expected AlterColumn")?;
  assert!(
    matches!(
      op,
      Operation::AlterColumn {
        changes,
        ..
      } if matches!(
        changes.as_slice(),
        [ColumnChange::CompositePrimaryKey { .. }]
      )
    ),
    "expected CompositePrimaryKey change, got {op:?}"
  );
  let sql = generate_sql_for(&ops, Dialect::Sqlite);
  assert!(
    sql.contains("PRAGMA foreign_keys = OFF"),
    "key-set change must recreate: {sql}"
  );
  assert!(
    sql.contains(r#"PRIMARY KEY ("memory_id", "tag")"#),
    "recreation must use new composite key: {sql}"
  );
  Ok(())
}

#[test]
fn autoincrement_flag_change_recreates_table() -> TestResult {
  let mut old_col = col("id", ColumnType::Integer, true, false);
  old_col.autoincrement = false;
  let mut new_col = old_col.clone();
  new_col.autoincrement = true;
  let old = Snapshot::from_registry(&SchemaRegistry::from_tables(vec![table(
    "retrieval_log",
    vec![old_col],
  )]));
  let new_reg = SchemaRegistry::from_tables(vec![table("retrieval_log", vec![new_col])]);
  let ops = diff_with_resolver(&old, &new_reg, &NoRenames)?;
  assert_eq!(ops.len(), 1);
  let op = ops.first().ok_or("expected AlterColumn")?;
  assert!(
    matches!(
      op,
      Operation::AlterColumn {
        changes,
        ..
      } if matches!(changes.as_slice(), [ColumnChange::Autoincrement { .. }])
    ),
    "expected Autoincrement change, got {op:?}"
  );
  let sql = generate_sql_for(&ops, Dialect::Sqlite);
  assert!(
    sql.contains("PRIMARY KEY AUTOINCREMENT"),
    "recreation must emit AUTOINCREMENT: {sql}"
  );
  Ok(())
}
