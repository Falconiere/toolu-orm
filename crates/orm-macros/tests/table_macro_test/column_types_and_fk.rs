//! Tests for new column types, FK actions, and strict mode.
//!
//! # Public API
//!
//! Tests: Uuid, Varchar, Boolean, Timestamp, Json, SmallInt, BigInt columns,
//! FK cascade/set_null actions, strict flag.

use toolu_orm_core::column::{
  BigInt, Boolean, ColumnType, ForeignKeyAction, Json, SmallInt, Text, Timestamp, Uuid, Varchar,
};
use toolu_orm_core::table::TableSchema;
use toolu_orm_macros::table;

#[table(name = "test_new_types")]
pub struct TestNewTypes {
  #[column(primary_key)]
  pub id: Uuid,
  pub name: Varchar<100>,
  #[column(not_null)]
  pub is_active: Boolean,
  pub created_at: Timestamp,
  pub metadata: Json,
  pub age: SmallInt,
  pub big_num: BigInt,
}

#[test]
fn test_new_column_types_in_macro() -> Result<(), Box<dyn std::error::Error>> {
  let def = TestNewTypes::table_def();
  assert_eq!(
    def.find_column("id").ok_or("missing id")?.column_type,
    ColumnType::Uuid
  );
  assert_eq!(
    def.find_column("name").ok_or("missing name")?.column_type,
    ColumnType::Varchar(100)
  );
  assert_eq!(
    def
      .find_column("is_active")
      .ok_or("missing is_active")?
      .column_type,
    ColumnType::Boolean
  );
  assert_eq!(
    def
      .find_column("created_at")
      .ok_or("missing created_at")?
      .column_type,
    ColumnType::Timestamp
  );
  assert_eq!(
    def
      .find_column("metadata")
      .ok_or("missing metadata")?
      .column_type,
    ColumnType::Json
  );
  assert_eq!(
    def.find_column("age").ok_or("missing age")?.column_type,
    ColumnType::SmallInt
  );
  assert_eq!(
    def
      .find_column("big_num")
      .ok_or("missing big_num")?
      .column_type,
    ColumnType::BigInt
  );
  assert!(!def.strict, "strict should default to false");
  Ok(())
}

#[table(name = "test_fk_actions")]
pub struct TestFkActions {
  #[column(primary_key)]
  pub id: Uuid,
  #[column(not_null, references = "parents(id)", on_delete = "cascade")]
  pub parent_id: Uuid,
  #[column(
    references = "groups(id)",
    on_delete = "set_null",
    on_update = "cascade"
  )]
  pub group_id: Uuid,
}

#[test]
fn test_fk_actions_in_macro() -> Result<(), Box<dyn std::error::Error>> {
  let def = TestFkActions::table_def();
  let parent = def.find_column("parent_id").ok_or("missing parent_id")?;
  assert_eq!(parent.references.as_deref(), Some("parents(id)"));
  assert_eq!(parent.on_delete, Some(ForeignKeyAction::Cascade));
  assert_eq!(parent.on_update, None);
  let group = def.find_column("group_id").ok_or("missing group_id")?;
  assert_eq!(group.on_delete, Some(ForeignKeyAction::SetNull));
  assert_eq!(group.on_update, Some(ForeignKeyAction::Cascade));
  Ok(())
}

#[table(name = "test_non_strict", strict = false)]
pub struct TestNonStrict {
  #[column(primary_key)]
  pub id: Text,
}

#[test]
fn test_strict_false_override() {
  let def = TestNonStrict::table_def();
  assert!(!def.strict, "strict should be false when explicitly set");
}
