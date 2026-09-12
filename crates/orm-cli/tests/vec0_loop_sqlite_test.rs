//! Generate → migrate against in-memory libsql for a `vec0` table: without
//! `sqlite-vec` the apply fails as [`MigrateError::MissingExtension`], an
//! identical regenerate is a no-op, and a dimension / metric change is refused
//! before a migration file is written. An FTS5-only migration on the same
//! driver still applies, so the mapping is not blanket.

use toolu_orm_cli::generate::run_generate;
use toolu_orm_cli::migrate::{run_migrate, MigrateError};
use toolu_orm_connection::{Database, LibsqlConnection};
use toolu_orm_core::column::{ColumnDef, ColumnType, VectorElement};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::fts5::Fts5Table;
use toolu_orm_core::schema::SchemaRegistry;
use toolu_orm_core::table::{TableDef, TableKind};
use toolu_orm_core::vec0::{
  DistanceMetric, Vec0AuxiliaryType, Vec0KeyType, Vec0MetadataType, Vec0Table,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn migrations_dir() -> Result<(tempfile::TempDir, String), Box<dyn std::error::Error>> {
  let tmp = tempfile::tempdir()?;
  let dir = tmp.path().join("migrations");
  std::fs::create_dir(&dir)?;
  let path = dir.to_str().ok_or("non-UTF8 path")?.to_owned();
  Ok((tmp, path))
}

fn memories() -> TableDef {
  let column = |name: &str, primary_key: bool| ColumnDef {
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
    autoincrement: false,
  };
  TableDef {
    name: "memories".to_owned(),
    columns: vec![column("id", true), column("body", false)],
    indexes: vec![],
    primary_key: vec![],
    strict: false,
    kind: TableKind::Ordinary,
  }
}

fn memory_vec(dim: u32, metric: DistanceMetric) -> TableDef {
  Vec0Table::new("memory_vec")
    .primary_key("memory_id", Vec0KeyType::Text)
    .vector_metric("embedding", VectorElement::Float, dim, metric)
    .partition_key("user_id", Vec0KeyType::Integer)
    .metadata("label", Vec0MetadataType::Text)
    .auxiliary("contents", Vec0AuxiliaryType::Text)
    .build_prevalidated()
}

fn vec0_registry(dim: u32, metric: DistanceMetric) -> SchemaRegistry {
  SchemaRegistry::from_tables(vec![memories(), memory_vec(dim, metric)])
}

fn memory_fts() -> TableDef {
  Fts5Table::new("memory_fts")
    .unindexed_column("memory_id", ColumnType::Text)
    .column("body", ColumnType::Text)
    .tokenize("porter unicode61")
    .build()
}

fn fts5_registry() -> SchemaRegistry {
  SchemaRegistry::from_tables(vec![memories(), memory_fts()])
}

async fn connect() -> Result<LibsqlConnection, Box<dyn std::error::Error>> {
  Ok(Database::init_local(":memory:").await?.connect()?)
}

async fn scalar(conn: &LibsqlConnection, sql: &str) -> Result<i64, Box<dyn std::error::Error>> {
  let mut rows = conn.inner_conn().query(sql, ()).await?;
  let row = rows.next().await?.ok_or("scalar query returned no row")?;
  Ok(row.get::<i64>(0)?)
}

#[tokio::test]
async fn migrating_vec0_without_the_extension_is_a_named_missing_module() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = connect().await?;
  run_generate(
    &vec0_registry(1024, DistanceMetric::Cosine),
    &dir,
    "init",
    Dialect::Sqlite,
  )?;

  let error = run_migrate(&conn, &dir, Dialect::Sqlite)
    .await
    .expect_err("vec0 DDL must fail without sqlite-vec");
  assert!(
    matches!(
      &error,
      MigrateError::MissingExtension { module, file }
        if module == "vec0" && file == "0001_init.sql"
    ),
    "expected MissingExtension for vec0, got {error}"
  );

  // ensure_migrations_table runs outside the migration transaction, so the
  // table exists; the failed migration's row must not.
  assert_eq!(
    scalar(
      &conn,
      "SELECT count(*) FROM _migrations WHERE name LIKE '%init%'"
    )
    .await?,
    0
  );
  assert_eq!(
    scalar(
      &conn,
      "SELECT count(*) FROM sqlite_master WHERE name = 'memories' AND type = 'table'"
    )
    .await?,
    0
  );
  assert_eq!(
    scalar(
      &conn,
      "SELECT count(*) FROM sqlite_master WHERE name = 'memory_vec'"
    )
    .await?,
    0
  );
  Ok(())
}

#[tokio::test]
async fn an_fts5_only_migration_still_applies_on_the_same_driver() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = connect().await?;
  run_generate(&fts5_registry(), &dir, "init", Dialect::Sqlite)?;
  run_migrate(&conn, &dir, Dialect::Sqlite).await?;
  assert_eq!(
    scalar(
      &conn,
      "SELECT count(*) FROM sqlite_master WHERE name = 'memory_fts' AND type = 'table'"
    )
    .await?,
    1
  );
  Ok(())
}

#[tokio::test]
async fn regenerating_the_same_vec0_schema_finds_no_change() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  run_generate(
    &vec0_registry(1024, DistanceMetric::Cosine),
    &dir,
    "init",
    Dialect::Sqlite,
  )?;
  assert_eq!(
    run_generate(
      &vec0_registry(1024, DistanceMetric::Cosine),
      &dir,
      "noop",
      Dialect::Sqlite
    )?,
    None
  );
  Ok(())
}

#[tokio::test]
async fn changing_the_dimension_is_refused_without_writing_a_migration() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  run_generate(
    &vec0_registry(1024, DistanceMetric::Cosine),
    &dir,
    "init",
    Dialect::Sqlite,
  )?;

  let error = run_generate(
    &vec0_registry(768, DistanceMetric::Cosine),
    &dir,
    "redim",
    Dialect::Sqlite,
  )
  .expect_err("expected the virtual-table change to be refused");
  let message = error.to_string();
  assert!(
    message.contains("memory_vec") && message.contains("columns"),
    "unhelpful error: {message}"
  );
  assert!(
    !std::path::Path::new(&dir).join("0002_redim.sql").exists(),
    "a migration was written for a refused change"
  );
  Ok(())
}

#[tokio::test]
async fn changing_the_distance_metric_is_refused_without_writing_a_migration() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  run_generate(
    &vec0_registry(1024, DistanceMetric::Cosine),
    &dir,
    "init",
    Dialect::Sqlite,
  )?;

  let error = run_generate(
    &vec0_registry(1024, DistanceMetric::L2),
    &dir,
    "remetric",
    Dialect::Sqlite,
  )
  .expect_err("expected the virtual-table change to be refused");
  let message = error.to_string();
  assert!(
    message.contains("memory_vec") && message.contains("module arguments"),
    "unhelpful error: {message}"
  );
  assert!(
    !std::path::Path::new(&dir)
      .join("0002_remetric.sql")
      .exists(),
    "a migration was written for a refused change"
  );
  Ok(())
}

#[test]
fn a_driver_message_without_a_module_name_stays_database() {
  let bare = MigrateError::from_statement("0001_init.sql", "no such module: ");
  assert!(
    matches!(
      &bare,
      MigrateError::Database(message)
        if message.contains("0001_init.sql") && message.contains("no such module:")
    ),
    "expected Database, got {bare}"
  );
  let no_marker = MigrateError::from_statement("0001_init.sql", "syntax error near vec0");
  assert!(
    matches!(&no_marker, MigrateError::Database(_)),
    "expected Database, got {no_marker}"
  );
  let wrapped =
    MigrateError::from_statement("0001_init.sql", "SQLite failure: `no such module: vec0`");
  assert!(
    matches!(
      &wrapped,
      MigrateError::MissingExtension { module, .. } if module == "vec0"
    ),
    "expected MissingExtension, got {wrapped}"
  );
  let bare_rusqlite = MigrateError::from_statement("0001_init.sql", "no such module: vec0");
  assert!(
    matches!(
      &bare_rusqlite,
      MigrateError::MissingExtension { module, .. } if module == "vec0"
    ),
    "expected MissingExtension, got {bare_rusqlite}"
  );
}
