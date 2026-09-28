//! Table-level composite FOREIGN KEY rendering in both dialects.

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::diff::{diff, Operation};
use toolu_orm_core::index::IndexDef;
use toolu_orm_core::snapshot::Snapshot;
use toolu_orm_core::sql::generate_sql_for;

use crate::composite_fk_schema::{evidence, evidence_fk, registry, work_items};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn position(sql: &str, needle: &str) -> Result<usize, String> {
  sql
    .find(needle)
    .ok_or_else(|| format!("missing {needle:?} in:\n{sql}"))
}

#[test]
fn sqlite_create_table_ends_with_unnamed_foreign_key_clauses() {
  let sql = generate_sql_for(
    &[Operation::CreateTable {
      table: evidence(vec![evidence_fk()]),
    }],
    Dialect::Sqlite,
  );
  assert!(
    sql.contains(
      r#"FOREIGN KEY ("work_item_id", "project_id") REFERENCES "project_work_items" ("id", "project_id") ON DELETE CASCADE"#
    ),
    "sql: {sql}"
  );
  assert!(!sql.contains("CONSTRAINT"), "sql: {sql}");
}

#[test]
fn sqlite_self_reference_renders_without_action() {
  let sql = generate_sql_for(
    &[Operation::CreateTable {
      table: work_items(),
    }],
    Dialect::Sqlite,
  );
  assert!(
    sql.contains(
      "FOREIGN KEY (\"parent_work_item_id\", \"project_id\") REFERENCES \"project_work_items\" (\"id\", \"project_id\")\n"
    ),
    "sql: {sql}"
  );
}

/// Postgres checks the referenced unique key when the constraint is created,
/// so every composite FK lands after the migration's indexes.
#[test]
fn postgres_attaches_foreign_keys_after_every_index() -> TestResult {
  let ops = diff(&Snapshot::empty(), &registry())?;
  let sql = generate_sql_for(&ops, Dialect::Postgres);
  let create = position(&sql, "CREATE TABLE IF NOT EXISTS \"project_work_items\"")?;
  let index = position(
    &sql,
    "CREATE UNIQUE INDEX IF NOT EXISTS \"project_work_items_id_project_uidx\"",
  )?;
  let self_fk = position(
    &sql,
    "ALTER TABLE \"project_work_items\" ADD CONSTRAINT \"fk_project_work_items_parent_work_item_id_project_id\" FOREIGN KEY (\"parent_work_item_id\", \"project_id\") REFERENCES \"project_work_items\" (\"id\", \"project_id\");",
  )?;
  let evidence_fk = position(
    &sql,
    "ALTER TABLE \"project_evidence\" ADD CONSTRAINT \"project_evidence_work_item_project_fk\" FOREIGN KEY (\"work_item_id\", \"project_id\") REFERENCES \"project_work_items\" (\"id\", \"project_id\") ON DELETE CASCADE;",
  )?;
  assert!(
    create < index && index < self_fk && index < evidence_fk,
    "sql:\n{sql}"
  );
  let create_sql = sql.get(create..index).ok_or("slice")?;
  assert!(!create_sql.contains("FOREIGN KEY"), "sql:\n{sql}");
  Ok(())
}

#[test]
fn postgres_add_foreign_key_follows_a_later_index() -> TestResult {
  let ops = vec![
    Operation::AddForeignKey {
      table: "project_evidence".to_owned(),
      fk: evidence_fk(),
    },
    Operation::CreateIndex {
      table: "project_work_items".to_owned(),
      index: IndexDef {
        name: "project_work_items_id_project_uidx".to_owned(),
        columns: vec!["id".into(), "project_id".into()],
        unique: true,
        where_clause: None,
      },
    },
  ];
  let sql = generate_sql_for(&ops, Dialect::Postgres);
  let index = position(&sql, "CREATE UNIQUE INDEX")?;
  let fk = position(
    &sql,
    "ADD CONSTRAINT \"project_evidence_work_item_project_fk\"",
  )?;
  assert!(index < fk, "sql:\n{sql}");
  Ok(())
}
