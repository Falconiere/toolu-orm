//! A table-level foreign key added, removed or changed on an existing table
//! rebuilds it on SQLite and becomes a constraint statement on Postgres.

use toolu_orm_core::column::ForeignKeyAction;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::diff::{diff, diff_with_resolver, ColumnChange, Operation};
use toolu_orm_core::rename::RenameResolver;
use toolu_orm_core::schema::SchemaRegistry;
use toolu_orm_core::snapshot::{ForeignKeyDef, Snapshot};
use toolu_orm_core::sql::generate_sql_for;
use toolu_orm_core::table::TableDef;

use crate::composite_fk_schema::{column, evidence, evidence_fk, projects, table, work_items};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn schema(evidence_fks: Vec<ForeignKeyDef>) -> SchemaRegistry {
  SchemaRegistry::from_tables(vec![projects(), work_items(), evidence(evidence_fks)])
}

/// The `TableForeignKeys` change of every `AlterColumn` in `ops`.
fn table_fk_changes(ops: &[Operation]) -> Vec<(Vec<ForeignKeyDef>, Vec<ForeignKeyDef>)> {
  let mut out = Vec::new();
  for op in ops {
    let Operation::AlterColumn { changes, .. } = op else {
      continue;
    };
    for change in changes {
      if let ColumnChange::TableForeignKeys { old, new } = change {
        out.push((old.clone(), new.clone()));
      }
    }
  }
  out
}

#[test]
fn adding_a_composite_fk_rebuilds_on_sqlite_and_adds_a_constraint_on_postgres() -> TestResult {
  let old = Snapshot::from_registry(&schema(vec![]));
  let ops = diff(&old, &schema(vec![evidence_fk()]))?;
  assert_eq!(table_fk_changes(&ops), vec![(vec![], vec![evidence_fk()])]);

  let sqlite = generate_sql_for(&ops, Dialect::Sqlite);
  assert!(
    sqlite.contains("CREATE TABLE \"_toolu_new_project_evidence\""),
    "{sqlite}"
  );
  assert!(!sqlite.contains("-- FOREIGN KEY"), "{sqlite}");
  assert!(
    sqlite.contains("FOREIGN KEY (\"work_item_id\", \"project_id\") REFERENCES \"project_work_items\" (\"id\", \"project_id\") ON DELETE CASCADE"),
    "{sqlite}"
  );

  let postgres = generate_sql_for(&ops, Dialect::Postgres);
  assert!(
    postgres.contains(
      "ALTER TABLE \"project_evidence\" ADD CONSTRAINT \"project_evidence_work_item_project_fk\""
    ),
    "{postgres}"
  );
  assert!(!postgres.contains("ALTER COLUMN"), "{postgres}");
  Ok(())
}

#[test]
fn removing_a_composite_fk_rebuilds_without_the_clause() -> TestResult {
  let old = Snapshot::from_registry(&schema(vec![evidence_fk()]));
  let ops = diff(&old, &schema(vec![]))?;
  assert_eq!(table_fk_changes(&ops), vec![(vec![evidence_fk()], vec![])]);

  let sqlite = generate_sql_for(&ops, Dialect::Sqlite);
  assert!(
    sqlite.contains("CREATE TABLE \"_toolu_new_project_evidence\""),
    "{sqlite}"
  );
  assert!(!sqlite.contains("-- drop FOREIGN KEY"), "{sqlite}");
  let staging = sqlite
    .split("CREATE TABLE \"_toolu_new_project_evidence\"")
    .nth(1)
    .and_then(|rest| rest.split(';').next())
    .ok_or("no staging table")?;
  assert!(!staging.contains("FOREIGN KEY"), "{sqlite}");

  let postgres = generate_sql_for(&ops, Dialect::Postgres);
  assert!(
    postgres.contains("DROP CONSTRAINT IF EXISTS \"project_evidence_work_item_project_fk\""),
    "{postgres}"
  );
  Ok(())
}

#[test]
fn changing_the_action_of_a_composite_fk_rebuilds() -> TestResult {
  let old = Snapshot::from_registry(&schema(vec![evidence_fk()]));
  let mut restrict = evidence_fk();
  restrict.on_delete = Some(ForeignKeyAction::Restrict);
  let ops = diff(&old, &schema(vec![restrict.clone()]))?;
  assert_eq!(
    table_fk_changes(&ops),
    vec![(vec![evidence_fk()], vec![restrict])]
  );
  let sqlite = generate_sql_for(&ops, Dialect::Sqlite);
  assert!(sqlite.contains("ON DELETE RESTRICT"), "{sqlite}");
  Ok(())
}

/// A per-column `references` change keeps its existing SQLite treatment.
#[test]
fn a_column_level_fk_change_is_not_a_table_level_one() -> TestResult {
  let plain = table(
    "notes",
    vec![column("id", true, true), column("project_id", false, true)],
    vec![],
  );
  let mut referencing = plain.clone();
  if let Some(c) = referencing.columns.get_mut(1) {
    c.references = Some("projects(id)".to_owned());
  }
  let old = Snapshot::from_registry(&SchemaRegistry::from_tables(vec![projects(), plain]));
  let ops = diff(
    &old,
    &SchemaRegistry::from_tables(vec![projects(), referencing]),
  )?;
  assert!(table_fk_changes(&ops).is_empty(), "{ops:?}");
  assert!(
    ops
      .iter()
      .any(|op| matches!(op, Operation::AddForeignKey { .. })),
    "{ops:?}"
  );
  Ok(())
}

struct RenameTable;

impl RenameResolver for RenameTable {
  fn resolve_tables(&self, _added: &[String], _removed: &[String]) -> Vec<(String, String)> {
    vec![("project_work_items".to_owned(), "work_items".to_owned())]
  }

  fn resolve_columns(&self, _: &str, _: &[String], _: &[String]) -> Vec<(String, String)> {
    vec![]
  }
}

/// Renaming a table renames its column-level FKs; that is not a table-level
/// FK change, so it must not rebuild the table.
#[test]
fn renaming_a_table_does_not_misread_its_column_fks() -> TestResult {
  let before = TableDef {
    foreign_keys: vec![],
    ..work_items()
  };
  let after = TableDef {
    name: "work_items".to_owned(),
    ..before.clone()
  };
  let old = Snapshot::from_registry(&SchemaRegistry::from_tables(vec![projects(), before]));
  let ops = diff_with_resolver(
    &old,
    &SchemaRegistry::from_tables(vec![projects(), after]),
    &RenameTable,
  )?;
  assert!(table_fk_changes(&ops).is_empty(), "{ops:?}");
  Ok(())
}
