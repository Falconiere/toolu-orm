//! The DDL each dialect generates, and the dialect recorded in the snapshot.

use toolu_orm_core::column::{ColumnDef, ColumnType};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::schema::SchemaRegistry;
use toolu_orm_core::table::{TableDef, TableKind};

use crate::support::TestResult;

#[test]
fn generate_creates_postgres_migration() -> TestResult {
  let dir = tempfile::tempdir()?;
  let mig_dir = dir.path().join("migrations");
  std::fs::create_dir(&mig_dir)?;
  let mig_path = mig_dir.to_str().ok_or("invalid path")?;

  let registry = SchemaRegistry::from_tables(vec![TableDef {
    name: "users".to_owned(),
    columns: vec![ColumnDef {
      name: "id".to_owned(),
      column_type: ColumnType::Uuid,
      primary_key: true,
      not_null: true,
      default: Some("uuid4_str()".to_owned()),
      unique: false,
      references: None,
      on_delete: None,
      on_update: None,
      check: None,
      unindexed: false,
      autoincrement: false,
    }],
    indexes: vec![],
    primary_key: vec![],
    strict: false,
    kind: TableKind::Ordinary,
    fts5_sync: None,
    row_security: None,
  }]);

  let result =
    toolu_orm_cli::generate::run_generate(&registry, mig_path, "init", Dialect::Postgres)?;
  assert!(result.is_some());

  let filename = result.ok_or("expected filename")?;
  let sql_path = mig_dir.join(&filename);
  let sql = std::fs::read_to_string(sql_path)?;
  assert!(sql.contains("UUID"));
  assert!(sql.contains("gen_random_uuid()"));
  assert!(!sql.contains("uuid4_str()"));
  Ok(())
}

#[test]
fn generate_creates_sqlite_migration() -> TestResult {
  let dir = tempfile::tempdir()?;
  let mig_dir = dir.path().join("migrations");
  std::fs::create_dir(&mig_dir)?;
  let mig_path = mig_dir.to_str().ok_or("invalid path")?;

  let registry = SchemaRegistry::from_tables(vec![TableDef {
    name: "users".to_owned(),
    columns: vec![ColumnDef {
      name: "id".to_owned(),
      column_type: ColumnType::Uuid,
      primary_key: true,
      not_null: true,
      default: Some("uuid4_str()".to_owned()),
      unique: false,
      references: None,
      on_delete: None,
      on_update: None,
      check: None,
      unindexed: false,
      autoincrement: false,
    }],
    indexes: vec![],
    primary_key: vec![],
    strict: false,
    kind: TableKind::Ordinary,
    fts5_sync: None,
    row_security: None,
  }]);

  let result = toolu_orm_cli::generate::run_generate(&registry, mig_path, "init", Dialect::Sqlite)?;
  assert!(result.is_some());

  let filename = result.ok_or("expected filename")?;
  let sql_path = mig_dir.join(&filename);
  let sql = std::fs::read_to_string(sql_path)?;
  assert!(sql.contains("TEXT"));
  assert!(sql.contains("uuid4_str()"));
  Ok(())
}

#[test]
fn generate_sets_dialect_in_snapshot() -> TestResult {
  let dir = tempfile::tempdir()?;
  let mig_dir = dir.path().join("migrations");
  std::fs::create_dir(&mig_dir)?;
  let mig_path = mig_dir.to_str().ok_or("invalid path")?;

  let registry = SchemaRegistry::from_tables(vec![TableDef {
    name: "items".to_owned(),
    columns: vec![ColumnDef {
      name: "id".to_owned(),
      column_type: ColumnType::Text,
      primary_key: true,
      not_null: true,
      default: None,
      unique: false,
      references: None,
      on_delete: None,
      on_update: None,
      check: None,
      unindexed: false,
      autoincrement: false,
    }],
    indexes: vec![],
    primary_key: vec![],
    strict: false,
    kind: TableKind::Ordinary,
    fts5_sync: None,
    row_security: None,
  }]);

  toolu_orm_cli::generate::run_generate(&registry, mig_path, "init", Dialect::Postgres)?;

  let snapshot_path = mig_dir.join("0001_init.snapshot.json");
  let content = std::fs::read_to_string(snapshot_path)?;
  assert!(
    content.contains("\"dialect\":\"postgres\"") || content.contains("\"dialect\": \"postgres\"")
  );
  Ok(())
}
