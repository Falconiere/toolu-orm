use toolu_orm_core::column::ColumnType;
use toolu_orm_core::diff::{diff_with_resolver, Operation};
use toolu_orm_core::rename::RenameResolver;
use toolu_orm_core::schema::SchemaRegistry;
use toolu_orm_core::snapshot::Snapshot;

use super::{col, table};

struct TestResolver {
  table_renames: Vec<(String, String)>,
  column_renames: Vec<(String, String)>,
}

impl RenameResolver for TestResolver {
  fn resolve_tables(&self, _added: &[String], _removed: &[String]) -> Vec<(String, String)> {
    self.table_renames.clone()
  }

  fn resolve_columns(
    &self,
    _table: &str,
    _added: &[String],
    _removed: &[String],
  ) -> Vec<(String, String)> {
    self.column_renames.clone()
  }
}

#[test]
fn diff_rename_table_via_resolver() {
  let old_reg = SchemaRegistry::from_tables(vec![table(
    "users",
    vec![col("id", ColumnType::Text, true, true)],
  )]);
  let old = Snapshot::from_registry(&old_reg);
  let new_reg = SchemaRegistry::from_tables(vec![table(
    "accounts",
    vec![col("id", ColumnType::Text, true, true)],
  )]);

  let resolver = TestResolver {
    table_renames: vec![("users".to_owned(), "accounts".to_owned())],
    column_renames: vec![],
  };

  let ops = diff_with_resolver(&old, &new_reg, &resolver).expect("diff should succeed");
  match ops.as_slice() {
    [Operation::RenameTable { old, new }] => {
      assert_eq!(old, "users");
      assert_eq!(new, "accounts");
    },
    _ => assert_eq!(ops.len(), 1, "expected exactly one RenameTable operation"),
  }
}

#[test]
fn diff_rename_column_via_resolver() {
  let old_reg = SchemaRegistry::from_tables(vec![table(
    "users",
    vec![
      col("id", ColumnType::Text, true, true),
      col("name", ColumnType::Text, false, false),
    ],
  )]);
  let old = Snapshot::from_registry(&old_reg);
  let new_reg = SchemaRegistry::from_tables(vec![table(
    "users",
    vec![
      col("id", ColumnType::Text, true, true),
      col("full_name", ColumnType::Text, false, false),
    ],
  )]);

  let resolver = TestResolver {
    table_renames: vec![],
    column_renames: vec![("name".to_owned(), "full_name".to_owned())],
  };

  let ops = diff_with_resolver(&old, &new_reg, &resolver).expect("diff should succeed");
  match ops.as_slice() {
    [Operation::RenameColumn { table, old, new }] => {
      assert_eq!(table, "users");
      assert_eq!(old, "name");
      assert_eq!(new, "full_name");
    },
    _ => assert_eq!(ops.len(), 1, "expected exactly one RenameColumn operation"),
  }
}
