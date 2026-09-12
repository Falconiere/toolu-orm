//! Table-level `#[primary_key(...)]` and `#[column(autoincrement)]`.

use toolu_orm_core::column::{ColumnType, Integer, Text};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::diff::Operation;
use toolu_orm_core::sql::generate_sql_for;
use toolu_orm_core::table::TableSchema;
use toolu_orm_macros::table;

#[table(name = "memory_tags")]
#[primary_key(memory_id, tag)]
pub struct MemoryTags {
  #[column(not_null)]
  pub memory_id: Text,
  #[column(not_null)]
  pub tag: Text,
}

#[table(name = "retrieval_log")]
pub struct RetrievalLog {
  #[column(primary_key, autoincrement)]
  pub id: Integer,
  pub note: Text,
}

#[test]
fn composite_primary_key_lands_in_table_def() {
  let def = MemoryTags::table_def();
  assert_eq!(def.primary_key, ["memory_id", "tag"]);
  assert!(!def.columns.iter().any(|c| c.primary_key));
  let sql = generate_sql_for(&[Operation::CreateTable { table: def }], Dialect::Sqlite);
  assert!(sql.contains(r#"PRIMARY KEY ("memory_id", "tag")"#), "{sql}");
}

#[test]
fn autoincrement_lands_in_column_def_and_sql() -> Result<(), Box<dyn std::error::Error>> {
  let def = RetrievalLog::table_def();
  let id = def.find_column("id").ok_or("missing id column")?;
  assert!(id.primary_key);
  assert!(id.autoincrement);
  assert_eq!(id.column_type, ColumnType::Integer);
  let sql = generate_sql_for(&[Operation::CreateTable { table: def }], Dialect::Sqlite);
  assert!(
    sql.contains(r#""id" INTEGER PRIMARY KEY AUTOINCREMENT"#),
    "{sql}"
  );
  Ok(())
}
