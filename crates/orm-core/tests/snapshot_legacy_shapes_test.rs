//! Legacy on-disk snapshot shape: `columns` as a JSON array, no `column_order` /
//! `indexes` / `foreign_keys` / `check_constraints` / `strict`.

use toolu_orm_core::column::{ColumnDef, ColumnType};
use toolu_orm_core::diff::{diff, Operation};
use toolu_orm_core::schema::SchemaRegistry;
use toolu_orm_core::snapshot::Snapshot;
use toolu_orm_core::table::{TableDef, TableKind};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn fixture_path() -> String {
  format!(
    "{}/tests/fixtures/legacy_snapshot.json",
    env!("CARGO_MANIFEST_DIR")
  )
}

fn users_column(name: &str, primary_key: bool, not_null: bool) -> ColumnDef {
  ColumnDef {
    name: name.to_owned(),
    column_type: ColumnType::Text,
    primary_key,
    not_null,
    default: None,
    unique: false,
    references: None,
    on_delete: None,
    on_update: None,
    check: None,
    unindexed: false,
    autoincrement: false,
  }
}

fn matching_registry() -> SchemaRegistry {
  let users = TableDef {
    name: "users".to_owned(),
    columns: vec![
      users_column("id", true, true),
      users_column("name", false, false),
    ],
    indexes: vec![],
    primary_key: vec![],
    strict: false,
    kind: toolu_orm_core::table::TableKind::Ordinary,
  };
  let mut author_id = users_column("author_id", false, true);
  author_id.references = Some("users(id)".to_owned());
  let posts = TableDef {
    name: "posts".to_owned(),
    columns: vec![users_column("id", true, true), author_id],
    indexes: vec![],
    primary_key: vec![],
    strict: false,
    kind: toolu_orm_core::table::TableKind::Ordinary,
  };
  SchemaRegistry::from_tables(vec![users, posts])
}

#[test]
fn legacy_array_columns_infer_column_order_and_defaults() -> TestResult {
  let snapshot = Snapshot::read_from_path(&fixture_path())?;

  let users = snapshot.tables.get("users").ok_or("missing users table")?;
  assert_eq!(users.column_order, vec!["id".to_owned(), "name".to_owned()]);
  assert!(users.indexes.is_empty());
  assert!(users.foreign_keys.is_empty());
  assert!(users.check_constraints.is_empty());
  assert!(!users.strict);

  let posts = snapshot.tables.get("posts").ok_or("missing posts table")?;
  assert_eq!(
    posts.column_order,
    vec!["id".to_owned(), "author_id".to_owned()]
  );
  assert!(posts.indexes.is_empty());
  assert!(posts.foreign_keys.is_empty());
  assert!(posts.check_constraints.is_empty());
  assert!(!posts.strict);

  Ok(())
}

#[test]
fn legacy_snapshot_columns_are_keyed_by_name() -> TestResult {
  let snapshot = Snapshot::read_from_path(&fixture_path())?;
  let users = snapshot.tables.get("users").ok_or("missing users table")?;
  assert!(users.columns.contains_key("id"));
  assert!(users.columns.contains_key("name"));
  let id = users.columns.get("id").ok_or("missing id column")?;
  assert!(id.primary_key);
  Ok(())
}

/// The fixture predates virtual tables: it has no `kind` on a table and no
/// `unindexed` on a column, and must still load as an ordinary table whose
/// columns are all indexed.
#[test]
fn legacy_tables_default_to_ordinary_kind_and_indexed_columns() -> TestResult {
  let snapshot = Snapshot::read_from_path(&fixture_path())?;
  for (name, table) in &snapshot.tables {
    assert_eq!(table.kind, TableKind::Ordinary, "{name} is not ordinary");
    for (column_name, column) in &table.columns {
      assert!(
        !column.unindexed,
        "{name}.{column_name} came back unindexed"
      );
    }
  }
  let restored = snapshot.to_registry();
  for table in restored.tables() {
    assert!(!table.is_virtual(), "{} became virtual", table.name);
  }
  Ok(())
}

#[test]
fn diff_against_legacy_snapshot_yields_single_add_foreign_key() -> TestResult {
  let legacy = Snapshot::read_from_path(&fixture_path())?;
  let registry = matching_registry();

  let ops = diff(&legacy, &registry).expect("diff should succeed");
  assert_eq!(ops.len(), 1);
  let op = ops.first().ok_or("expected one operation")?;
  let Operation::AddForeignKey { table, fk } = op else {
    return Err(format!("expected AddForeignKey, got {op:?}").into());
  };
  assert_eq!(table, "posts");
  assert_eq!(fk.columns, vec!["author_id".to_owned()]);
  assert_eq!(fk.references_table, "users");
  assert_eq!(fk.references_columns, vec!["id".to_owned()]);

  Ok(())
}
