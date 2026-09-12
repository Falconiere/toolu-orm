//! Tests for ColumnEnum derive and index parsing.
//!
//! # Public API
//!
//! Tests: ColumnEnum variants, enum CHECK constraints, index/unique_index
//! parsing, full cross-crate integration (PipelineRun schema + SQL).

use toolu_orm_core::column::{
  ColumnType, EnumSchema, ForeignKeyAction, Text, Timestamp, Uuid, Varchar,
};
use toolu_orm_core::index::IndexColumn;
use toolu_orm_core::table::TableSchema;
use toolu_orm_macros::{table, ColumnEnum};

use super::schema_basics::ConversationObjectDefaultText;

// --- ColumnEnum derive macro ---

#[derive(serde::Serialize, serde::Deserialize, ColumnEnum)]
#[serde(rename_all = "snake_case")]
pub enum TestStatus {
  Draft,
  Active,
  Archived,
}

#[test]
fn test_column_enum_variants() {
  let variants = TestStatus::variants();
  assert_eq!(variants, &["draft", "active", "archived"]);
}

#[table(name = "test_enum_table")]
pub struct TestEnumTable {
  #[column(primary_key)]
  pub id: Uuid,
  #[column(not_null, default = "'draft'")]
  pub status: TestStatus,
}

#[test]
fn test_enum_field_generates_check() -> Result<(), Box<dyn std::error::Error>> {
  let def = TestEnumTable::table_def();
  let status = def.find_column("status").ok_or("missing status column")?;
  assert_eq!(status.column_type, ColumnType::Text);
  assert!(status
    .check
    .as_ref()
    .ok_or("missing check")?
    .contains("'draft'"));
  assert!(status
    .check
    .as_ref()
    .ok_or("missing check")?
    .contains("'active'"));
  assert!(status
    .check
    .as_ref()
    .ok_or("missing check")?
    .contains("'archived'"));
  Ok(())
}

#[test]
fn test_object_type_now_generates_check() -> Result<(), Box<dyn std::error::Error>> {
  let def = ConversationObjectDefaultText::table_def();
  let col = def
    .find_column("object_type")
    .ok_or("missing object_type column")?;
  assert_eq!(col.column_type, ColumnType::Text);
  // ObjectType now has ColumnEnum, so it should generate CHECK
  assert!(col.check.is_some());
  Ok(())
}

// --- Index parsing ---

#[table(name = "test_indexes")]
#[index("idx_test_name", name)]
#[index("idx_test_composite", name, repo_id)]
#[unique_index("idx_test_unique", email)]
pub struct TestIndexes {
  #[column(primary_key)]
  pub id: Uuid,
  pub name: Text,
  pub repo_id: Uuid,
  pub email: Varchar<255>,
}

#[test]
fn test_indexes_in_table_def() -> Result<(), Box<dyn std::error::Error>> {
  let def = TestIndexes::table_def();
  assert_eq!(def.indexes.len(), 3);

  let idx = def
    .indexes
    .iter()
    .find(|i| i.name == "idx_test_name")
    .ok_or("missing idx_test_name")?;
  assert_eq!(idx.columns, vec![IndexColumn::new("name")]);
  assert!(!idx.unique);

  let composite = def
    .indexes
    .iter()
    .find(|i| i.name == "idx_test_composite")
    .ok_or("missing idx_test_composite")?;
  assert_eq!(
    composite.columns,
    vec![IndexColumn::new("name"), IndexColumn::new("repo_id")]
  );
  assert!(!composite.unique);

  let unique = def
    .indexes
    .iter()
    .find(|i| i.name == "idx_test_unique")
    .ok_or("missing idx_test_unique")?;
  assert_eq!(unique.columns, vec![IndexColumn::new("email")]);
  assert!(unique.unique);
  Ok(())
}

// --- Full cross-crate integration test ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, ColumnEnum)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
  Pending,
  Running,
  Success,
  Failed,
}

#[table(name = "pipeline_runs", strict = true)]
#[index("idx_runs_pipeline", pipeline_id)]
#[index("idx_runs_status", status)]
pub struct PipelineRun {
  #[column(primary_key, default = "uuid4_str()")]
  pub id: Uuid,
  #[column(not_null, references = "pipelines(id)", on_delete = "cascade")]
  pub pipeline_id: Uuid,
  #[column(not_null, default = "'pending'")]
  pub status: RunStatus,
  #[column(not_null, default = "datetime('now')")]
  pub created_at: Timestamp,
}

#[test]
fn test_full_pipeline_run_schema() -> Result<(), Box<dyn std::error::Error>> {
  let def = PipelineRun::table_def();
  assert!(def.strict);
  assert_eq!(def.name, "pipeline_runs");
  assert_eq!(def.columns.len(), 4);
  assert_eq!(def.indexes.len(), 2);

  let id = def.find_column("id").ok_or("missing id column")?;
  assert_eq!(id.column_type, ColumnType::Uuid);
  assert!(id.primary_key);
  assert_eq!(id.default.as_deref(), Some("uuid4_str()"));

  let pipeline_id = def
    .find_column("pipeline_id")
    .ok_or("missing pipeline_id column")?;
  assert_eq!(pipeline_id.on_delete, Some(ForeignKeyAction::Cascade));
  assert_eq!(pipeline_id.references.as_deref(), Some("pipelines(id)"));

  let status = def.find_column("status").ok_or("missing status column")?;
  assert_eq!(status.column_type, ColumnType::Text);
  assert!(status
    .check
    .as_ref()
    .ok_or("missing check")?
    .contains("'pending'"));
  assert!(status
    .check
    .as_ref()
    .ok_or("missing check")?
    .contains("'failed'"));

  // Verify SQL generation from the table def
  use toolu_orm_core::dialect::Dialect;
  use toolu_orm_core::diff::Operation;
  use toolu_orm_core::sql::generate_sql_for;
  let sql = generate_sql_for(
    &[Operation::CreateTable { table: def.clone() }],
    Dialect::Sqlite,
  );
  assert!(sql.contains("STRICT"), "sql: {sql}");
  assert!(sql.contains("ON DELETE CASCADE"), "sql: {sql}");
  assert!(sql.contains("CHECK"), "sql: {sql}");
  Ok(())
}
