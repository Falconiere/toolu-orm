use toolu_orm_core::column::ColumnType;
use toolu_orm_core::diff::{diff_with_resolver, ColumnChange, Operation};
use toolu_orm_core::rename::NoRenames;
use toolu_orm_core::schema::SchemaRegistry;
use toolu_orm_core::snapshot::Snapshot;

use super::{col, table};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn test_diff_create_table_from_empty() -> TestResult {
  let old = Snapshot::from_registry(&SchemaRegistry::from_tables(vec![]));
  let new_reg = SchemaRegistry::from_tables(vec![table(
    "conversations",
    vec![col("id", ColumnType::Text, true, false)],
  )]);
  let ops = diff_with_resolver(&old, &new_reg, &NoRenames).expect("diff should succeed");
  assert_eq!(ops.len(), 1);
  let op = ops.first().ok_or("expected one operation")?;
  assert!(
    matches!(op, Operation::CreateTable { table: t } if t.name == "conversations"),
    "expected CreateTable for conversations, got {op:?}"
  );
  Ok(())
}

#[test]
fn test_diff_drop_table() -> TestResult {
  let old_reg = SchemaRegistry::from_tables(vec![table(
    "old_table",
    vec![col("id", ColumnType::Text, true, false)],
  )]);
  let old = Snapshot::from_registry(&old_reg);
  let new_reg = SchemaRegistry::from_tables(vec![]);
  let ops = diff_with_resolver(&old, &new_reg, &NoRenames).expect("diff should succeed");
  assert_eq!(ops.len(), 1);
  let op = ops.first().ok_or("expected one operation")?;
  assert!(
    matches!(op, Operation::DropTable { name } if name == "old_table"),
    "expected DropTable for old_table, got {op:?}"
  );
  Ok(())
}

#[test]
fn test_diff_add_column() -> TestResult {
  let old_reg = SchemaRegistry::from_tables(vec![table(
    "conversations",
    vec![col("id", ColumnType::Text, true, false)],
  )]);
  let old = Snapshot::from_registry(&old_reg);
  let new_reg = SchemaRegistry::from_tables(vec![table(
    "conversations",
    vec![
      col("id", ColumnType::Text, true, false),
      col("title", ColumnType::Text, false, false),
    ],
  )]);
  let ops = diff_with_resolver(&old, &new_reg, &NoRenames).expect("diff should succeed");
  assert_eq!(ops.len(), 1);
  let op = ops.first().ok_or("expected one operation")?;
  assert!(
    matches!(op, Operation::AddColumn { table: t, column: c }
      if t == "conversations" && c.name == "title"),
    "expected AddColumn title on conversations, got {op:?}"
  );
  Ok(())
}

#[test]
fn test_diff_drop_column() -> TestResult {
  let old_reg = SchemaRegistry::from_tables(vec![table(
    "conversations",
    vec![
      col("id", ColumnType::Text, true, false),
      col("title", ColumnType::Text, false, false),
    ],
  )]);
  let old = Snapshot::from_registry(&old_reg);
  let new_reg = SchemaRegistry::from_tables(vec![table(
    "conversations",
    vec![col("id", ColumnType::Text, true, false)],
  )]);
  let ops = diff_with_resolver(&old, &new_reg, &NoRenames).expect("diff should succeed");
  assert_eq!(ops.len(), 1);
  let op = ops.first().ok_or("expected one operation")?;
  assert!(
    matches!(op, Operation::DropColumn { table: t, column: c }
      if t == "conversations" && c == "title"),
    "expected DropColumn title on conversations, got {op:?}"
  );
  Ok(())
}

#[test]
fn test_diff_alter_column() -> TestResult {
  let old_reg = SchemaRegistry::from_tables(vec![table(
    "conversations",
    vec![col("title", ColumnType::Text, false, false)],
  )]);
  let old = Snapshot::from_registry(&old_reg);
  let new_reg = SchemaRegistry::from_tables(vec![table(
    "conversations",
    vec![col("title", ColumnType::Text, false, true)],
  )]);
  let ops = diff_with_resolver(&old, &new_reg, &NoRenames).expect("diff should succeed");
  assert_eq!(ops.len(), 1);
  let op = ops.first().ok_or("expected one operation")?;
  assert!(
    matches!(op, Operation::AlterColumn { table: t, ref changes, .. }
      if t == "conversations" && changes.iter().any(|c| matches!(c, ColumnChange::Nullable { column, old: true, new: false, .. } if column == "title"))),
    "expected AlterColumn for title on conversations, got {op:?}"
  );
  Ok(())
}

#[test]
fn test_diff_alter_column_type_change() {
  let old_reg = SchemaRegistry::from_tables(vec![table(
    "users",
    vec![col("age", ColumnType::Integer, false, false)],
  )]);
  let old = Snapshot::from_registry(&old_reg);
  let new_reg = SchemaRegistry::from_tables(vec![table(
    "users",
    vec![col("age", ColumnType::BigInt, false, false)],
  )]);
  let ops = diff_with_resolver(&old, &new_reg, &NoRenames).expect("diff should succeed");
  match ops.as_slice() {
    [Operation::AlterColumn {
      table,
      changes,
      table_def,
    }] => {
      assert_eq!(table, "users");
      assert_eq!(table_def.name, "users");
      match changes.as_slice() {
        [ColumnChange::Type { column, .. }] => assert_eq!(column, "age"),
        _ => assert_eq!(
          changes.len(),
          1,
          "expected one Type change, got {changes:?}"
        ),
      }
    },
    [wrong] => assert!(
      matches!(wrong, Operation::AlterColumn { .. }),
      "expected AlterColumn, got {wrong:?}"
    ),
    _ => assert_eq!(ops.len(), 1, "expected exactly one operation"),
  }
}

#[test]
fn test_diff_alter_column_multiple_changes_batched() {
  let mut old_col = col("email", ColumnType::Text, false, false);
  old_col.unique = false;
  let mut new_col = col("email", ColumnType::Text, false, true);
  new_col.unique = true;

  let old_reg = SchemaRegistry::from_tables(vec![table("users", vec![old_col])]);
  let old = Snapshot::from_registry(&old_reg);
  let new_reg = SchemaRegistry::from_tables(vec![table("users", vec![new_col])]);
  let ops = diff_with_resolver(&old, &new_reg, &NoRenames).expect("diff should succeed");
  match ops.as_slice() {
    [Operation::AlterColumn {
      changes, table_def, ..
    }] => {
      assert_eq!(changes.len(), 2);
      assert!(changes
        .iter()
        .any(|c| matches!(c, ColumnChange::Nullable { .. })));
      assert!(changes
        .iter()
        .any(|c| matches!(c, ColumnChange::Unique { .. })));
      assert_eq!(table_def.name, "users");
    },
    [wrong] => assert!(
      matches!(wrong, Operation::AlterColumn { .. }),
      "expected AlterColumn, got {wrong:?}"
    ),
    _ => assert_eq!(ops.len(), 1, "expected exactly one operation"),
  }
}

#[test]
fn test_diff_no_changes() {
  let reg = SchemaRegistry::from_tables(vec![table(
    "conversations",
    vec![col("id", ColumnType::Text, true, false)],
  )]);
  let snap = Snapshot::from_registry(&reg);
  let ops = diff_with_resolver(&snap, &reg, &NoRenames).expect("diff should succeed");
  assert!(ops.is_empty());
}

#[test]
fn test_diff_multiple_operations() {
  let old_reg = SchemaRegistry::from_tables(vec![
    table(
      "conversations",
      vec![col("id", ColumnType::Text, true, false)],
    ),
    table("to_drop", vec![col("id", ColumnType::Text, true, false)]),
  ]);
  let old = Snapshot::from_registry(&old_reg);
  let new_reg = SchemaRegistry::from_tables(vec![
    table(
      "conversations",
      vec![
        col("id", ColumnType::Text, true, false),
        col("title", ColumnType::Text, false, false),
      ],
    ),
    table(
      "new_table",
      vec![col("id", ColumnType::Integer, true, false)],
    ),
  ]);
  let ops = diff_with_resolver(&old, &new_reg, &NoRenames).expect("diff should succeed");
  assert_eq!(ops.len(), 3);
}
