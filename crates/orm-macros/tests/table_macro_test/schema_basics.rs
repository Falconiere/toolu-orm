//! Basic table schema generation tests.
//!
//! # Public API
//!
//! Tests: table_def generation, primary key, not_null, default, plain columns,
//! as_text, custom type defaults.

use toolu_orm_core::column::{ColumnType, Integer, Text};
use toolu_orm_core::table::TableSchema;
use toolu_orm_macros::{table, ColumnEnum};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ColumnEnum)]
pub enum ObjectType {
  Pipeline,
  Repository,
}

#[table(name = "conversations")]
pub struct Conversation {
  #[column(primary_key)]
  pub id: Text,

  #[column(not_null)]
  pub pipeline_id: Text,

  pub title: Text,

  #[column(not_null, default = "unixepoch()")]
  pub created_at: Integer,
}

#[table(name = "conversation_objects")]
pub struct ConversationObject {
  #[column(primary_key)]
  pub id: Text,

  #[column(not_null, as_text)]
  pub object_type: ObjectType,
}

#[table(name = "conversation_objects_default_text")]
pub struct ConversationObjectDefaultText {
  #[column(primary_key)]
  pub id: Text,

  pub object_type: ObjectType,
}

#[test]
fn test_table_macro_generates_table_def() {
  let def = Conversation::table_def();
  assert_eq!(def.name, "conversations");
  assert_eq!(def.columns.len(), 4);
}

#[test]
fn test_table_macro_primary_key_column() -> TestResult {
  let def = Conversation::table_def();
  let id_col = def.find_column("id").ok_or("id column must exist")?;
  assert!(id_col.primary_key);
  assert_eq!(id_col.column_type, ColumnType::Text);
  Ok(())
}

#[test]
fn test_table_macro_not_null_column() -> TestResult {
  let def = Conversation::table_def();
  let col = def
    .find_column("pipeline_id")
    .ok_or("pipeline_id column must exist")?;
  assert!(col.not_null);
  assert!(!col.primary_key);
  Ok(())
}

#[test]
fn test_table_macro_default_column() -> TestResult {
  let def = Conversation::table_def();
  let col = def
    .find_column("created_at")
    .ok_or("created_at column must exist")?;
  assert!(col.not_null);
  assert_eq!(col.default.as_deref(), Some("unixepoch()"));
  Ok(())
}

#[test]
fn test_table_macro_plain_column() -> TestResult {
  let def = Conversation::table_def();
  let col = def.find_column("title").ok_or("title column must exist")?;
  assert!(!col.not_null);
  assert!(!col.primary_key);
  assert!(!col.unique);
  assert!(col.default.is_none());
  Ok(())
}

#[test]
fn test_table_macro_as_text_column_for_custom_type() -> TestResult {
  let def = ConversationObject::table_def();
  let col = def
    .find_column("object_type")
    .ok_or("object_type column must exist")?;
  assert_eq!(col.column_type, ColumnType::Text);
  assert!(col.not_null);
  Ok(())
}

#[test]
fn test_table_macro_custom_type_defaults_to_text() -> TestResult {
  let def = ConversationObjectDefaultText::table_def();
  let col = def
    .find_column("object_type")
    .ok_or("object_type column must exist")?;
  assert_eq!(col.column_type, ColumnType::Text);
  Ok(())
}
