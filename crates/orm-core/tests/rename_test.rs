use toolu_orm_core::column::{ColumnDef, ColumnType};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::diff::{diff_with_resolver, Operation};
use toolu_orm_core::rename::{NoRenames, RenameResolver};
use toolu_orm_core::schema::SchemaRegistry;
use toolu_orm_core::snapshot::Snapshot;
use toolu_orm_core::sql::generate_sql_for;
use toolu_orm_core::table::TableDef;

#[test]
fn no_renames_returns_empty_table_renames() {
  let resolver = NoRenames;
  let added = vec!["accounts".to_owned()];
  let removed = vec!["users".to_owned()];
  let result = resolver.resolve_tables(&added, &removed);
  assert!(result.is_empty());
}

#[test]
fn no_renames_returns_empty_column_renames() {
  let resolver = NoRenames;
  let added = vec!["full_name".to_owned()];
  let removed = vec!["name".to_owned()];
  let result = resolver.resolve_columns("users", &added, &removed);
  assert!(result.is_empty());
}

#[test]
fn no_renames_with_empty_inputs() {
  let resolver = NoRenames;
  let result = resolver.resolve_tables(&[], &[]);
  assert!(result.is_empty());
  let result = resolver.resolve_columns("any_table", &[], &[]);
  assert!(result.is_empty());
}

// ── Fixtures ───────────────────────────────────────────────────────────────

fn text_column(name: &str, primary_key: bool) -> ColumnDef {
  ColumnDef {
    name: name.to_owned(),
    column_type: ColumnType::Text,
    primary_key,
    not_null: primary_key,
    default: None,
    unique: false,
    references: None,
    on_delete: None,
    on_update: None,
    check: None,
    unindexed: false,
  }
}

fn registry_with_table(table_name: &str, column_names: &[&str]) -> SchemaRegistry {
  let columns = column_names
    .iter()
    .enumerate()
    .map(|(i, name)| text_column(name, i == 0))
    .collect();
  SchemaRegistry::from_tables(vec![TableDef {
    name: table_name.to_owned(),
    columns,
    indexes: vec![],
    strict: false,
    kind: toolu_orm_core::table::TableKind::Ordinary,
  }])
}

// ── Resolvers ────────────────────────────────────────────────────────────────

struct TableRenameResolver {
  old: &'static str,
  new: &'static str,
}

impl RenameResolver for TableRenameResolver {
  fn resolve_tables(&self, added: &[String], removed: &[String]) -> Vec<(String, String)> {
    if added.iter().any(|a| a == self.new) && removed.iter().any(|r| r == self.old) {
      vec![(self.old.to_owned(), self.new.to_owned())]
    } else {
      vec![]
    }
  }

  fn resolve_columns(
    &self,
    _table: &str,
    _added: &[String],
    _removed: &[String],
  ) -> Vec<(String, String)> {
    vec![]
  }
}

struct ColumnRenameResolver {
  table: &'static str,
  old: &'static str,
  new: &'static str,
}

impl RenameResolver for ColumnRenameResolver {
  fn resolve_tables(&self, _added: &[String], _removed: &[String]) -> Vec<(String, String)> {
    vec![]
  }

  fn resolve_columns(
    &self,
    table: &str,
    added: &[String],
    removed: &[String],
  ) -> Vec<(String, String)> {
    if table == self.table
      && added.iter().any(|a| a == self.new)
      && removed.iter().any(|r| r == self.old)
    {
      vec![(self.old.to_owned(), self.new.to_owned())]
    } else {
      vec![]
    }
  }
}

// ── Table rename ───────────────────────────────────────────────────────────

#[test]
fn table_rename_produces_rename_table_op_and_no_create_drop_table() {
  let old_registry = registry_with_table("users", &["id", "name"]);
  let old_snapshot = Snapshot::from_registry(&old_registry);
  let new_registry = registry_with_table("people", &["id", "name"]);
  let resolver = TableRenameResolver {
    old: "users",
    new: "people",
  };

  let ops =
    diff_with_resolver(&old_snapshot, &new_registry, &resolver).expect("diff should succeed");

  assert!(ops.iter().any(|op| matches!(
    op,
    Operation::RenameTable { old, new } if old == "users" && new == "people"
  )));
  assert!(!ops.iter().any(|op| matches!(
    op,
    Operation::CreateTable { .. } | Operation::DropTable { .. }
  )));
}

#[test]
fn table_rename_sql_contains_alter_table_rename_to() {
  let old_registry = registry_with_table("users", &["id", "name"]);
  let old_snapshot = Snapshot::from_registry(&old_registry);
  let new_registry = registry_with_table("people", &["id", "name"]);
  let resolver = TableRenameResolver {
    old: "users",
    new: "people",
  };

  let ops =
    diff_with_resolver(&old_snapshot, &new_registry, &resolver).expect("diff should succeed");

  let sqlite_sql = generate_sql_for(&ops, Dialect::Sqlite);
  assert!(sqlite_sql.contains(r#"ALTER TABLE "users" RENAME TO "people""#));

  let postgres_sql = generate_sql_for(&ops, Dialect::Postgres);
  assert!(postgres_sql.contains(r#"ALTER TABLE "users" RENAME TO "people""#));
}

// ── Column rename ────────────────────────────────────────────────────────────

#[test]
fn column_rename_produces_rename_column_op_and_no_add_drop_column() {
  let old_registry = registry_with_table("users", &["id", "name"]);
  let old_snapshot = Snapshot::from_registry(&old_registry);
  let new_registry = registry_with_table("users", &["id", "full_name"]);
  let resolver = ColumnRenameResolver {
    table: "users",
    old: "name",
    new: "full_name",
  };

  let ops =
    diff_with_resolver(&old_snapshot, &new_registry, &resolver).expect("diff should succeed");

  assert!(ops.iter().any(|op| matches!(
    op,
    Operation::RenameColumn { table, old, new }
      if table == "users" && old == "name" && new == "full_name"
  )));
  assert!(!ops.iter().any(|op| matches!(
    op,
    Operation::AddColumn { .. } | Operation::DropColumn { .. }
  )));
}

#[test]
fn column_rename_sql_contains_rename_column_to() {
  let old_registry = registry_with_table("users", &["id", "name"]);
  let old_snapshot = Snapshot::from_registry(&old_registry);
  let new_registry = registry_with_table("users", &["id", "full_name"]);
  let resolver = ColumnRenameResolver {
    table: "users",
    old: "name",
    new: "full_name",
  };

  let ops =
    diff_with_resolver(&old_snapshot, &new_registry, &resolver).expect("diff should succeed");

  let sqlite_sql = generate_sql_for(&ops, Dialect::Sqlite);
  assert!(sqlite_sql.contains(r#"ALTER TABLE "users" RENAME COLUMN "name" TO "full_name""#));

  let postgres_sql = generate_sql_for(&ops, Dialect::Postgres);
  assert!(postgres_sql.contains(r#"ALTER TABLE "users" RENAME COLUMN "name" TO "full_name""#));
}

// ── Empty resolver still falls back to drop + create ─────────────────────────

#[test]
fn empty_resolver_yields_drop_and_create_instead_of_rename() {
  let old_registry = registry_with_table("users", &["id", "name"]);
  let old_snapshot = Snapshot::from_registry(&old_registry);
  let new_registry = registry_with_table("people", &["id", "name"]);

  let ops =
    diff_with_resolver(&old_snapshot, &new_registry, &NoRenames).expect("diff should succeed");

  assert!(ops
    .iter()
    .any(|op| matches!(op, Operation::DropTable { name } if name == "users")));
}
