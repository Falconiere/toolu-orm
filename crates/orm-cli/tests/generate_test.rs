use toolu_orm_core::column::{ColumnDef, ColumnType};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::journal::Journal;
use toolu_orm_core::schema::SchemaRegistry;
use toolu_orm_core::table::{TableDef, TableKind};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn col(name: &str, ct: ColumnType, pk: bool, nn: bool) -> ColumnDef {
  ColumnDef {
    name: name.to_owned(),
    column_type: ct,
    primary_key: pk,
    not_null: nn,
    default: None,
    unique: false,
    references: None,
    on_delete: None,
    on_update: None,
    check: None,
    unindexed: false,
  }
}

fn sample_registry() -> SchemaRegistry {
  SchemaRegistry::from_tables(vec![TableDef {
    name: "conversations".to_owned(),
    columns: vec![
      col("id", ColumnType::Text, true, false),
      col("title", ColumnType::Text, false, false),
    ],
    indexes: vec![],
    strict: false,
    kind: TableKind::Ordinary,
  }])
}

#[test]
fn test_generate_creates_journal_and_snapshot() -> TestResult {
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
      not_null: false,
      default: Some("uuid4_str()".to_owned()),
      unique: false,
      references: None,
      on_delete: None,
      on_update: None,
      check: None,
      unindexed: false,
    }],
    indexes: vec![],
    strict: true,
    kind: TableKind::Ordinary,
  }]);

  let result =
    toolu_orm_cli::generate::run_generate(&registry, mig_path, "initial", Dialect::Sqlite)?;
  assert!(result.is_some());
  let filename = result.ok_or("expected filename")?;
  assert_eq!(filename, "0001_initial.sql");

  // Journal should exist and have one entry
  let journal_path = mig_dir.join("_journal.json");
  let journal = Journal::read_from_path(journal_path.to_str().ok_or("invalid path")?)?;
  assert_eq!(journal.entries.len(), 1);
  let entry = journal.entries.first().ok_or("expected entry")?;
  assert_eq!(entry.name, "0001_initial.sql");
  assert!(entry.hash.starts_with("sha256:"));

  // Snapshot file should exist
  let snapshot_path = mig_dir.join("0001_initial.snapshot.json");
  assert!(snapshot_path.exists());
  Ok(())
}

#[test]
fn test_generate_incremental_migration() -> TestResult {
  let dir = tempfile::tempdir()?;
  let mig_dir = dir.path().join("migrations");
  std::fs::create_dir(&mig_dir)?;
  let mig_path = mig_dir.to_str().ok_or("invalid path")?;

  // First: generate initial
  let initial = SchemaRegistry::from_tables(vec![TableDef {
    name: "conversations".to_owned(),
    columns: vec![col("id", ColumnType::Text, true, false)],
    indexes: vec![],
    strict: false,
    kind: TableKind::Ordinary,
  }]);
  toolu_orm_cli::generate::run_generate(&initial, mig_path, "initial", Dialect::Sqlite)?;

  // Second: add a column
  let updated = sample_registry();
  let filename =
    toolu_orm_cli::generate::run_generate(&updated, mig_path, "add_title", Dialect::Sqlite)?;
  assert_eq!(filename, Some("0002_add_title.sql".to_owned()));

  let content = std::fs::read_to_string(mig_dir.join("0002_add_title.sql"))?;
  assert!(content.contains("ADD COLUMN"));

  // Journal should have 2 entries
  let journal_path = mig_dir.join("_journal.json");
  let journal = Journal::read_from_path(journal_path.to_str().ok_or("invalid path")?)?;
  assert_eq!(journal.entries.len(), 2);
  Ok(())
}

#[test]
fn test_no_changes_detected() -> TestResult {
  let dir = tempfile::tempdir()?;
  let mig_dir = dir.path().join("migrations");
  std::fs::create_dir(&mig_dir)?;
  let mig_path = mig_dir.to_str().ok_or("invalid path")?;

  let registry = sample_registry();
  toolu_orm_cli::generate::run_generate(&registry, mig_path, "initial", Dialect::Sqlite)?;

  // Run again with same schema — should return None
  let result =
    toolu_orm_cli::generate::run_generate(&registry, mig_path, "no_changes", Dialect::Sqlite)?;
  assert_eq!(result, None);
  Ok(())
}

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
    }],
    indexes: vec![],
    strict: false,
    kind: TableKind::Ordinary,
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
    }],
    indexes: vec![],
    strict: false,
    kind: TableKind::Ordinary,
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
    }],
    indexes: vec![],
    strict: false,
    kind: TableKind::Ordinary,
  }]);

  toolu_orm_cli::generate::run_generate(&registry, mig_path, "init", Dialect::Postgres)?;

  let snapshot_path = mig_dir.join("0001_init.snapshot.json");
  let content = std::fs::read_to_string(snapshot_path)?;
  assert!(
    content.contains("\"dialect\":\"postgres\"") || content.contains("\"dialect\": \"postgres\"")
  );
  Ok(())
}
