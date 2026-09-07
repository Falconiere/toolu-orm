use toolu_orm_connection::{Database, DbConnection, LibsqlConnection};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::journal::{compute_hash, Journal};
use toolu_orm_core::row::FromRow;
use toolu_orm_core::value::Value;

type TestResult = Result<(), Box<dyn std::error::Error>>;

async fn create_test_connection() -> Result<LibsqlConnection, Box<dyn std::error::Error>> {
  let db = Database::init_local(":memory:").await?;
  Ok(db.connect()?)
}

#[tokio::test]
async fn test_migrate_applies_pending_migrations_legacy() -> TestResult {
  let dir = tempfile::tempdir()?;
  let migrations_dir = dir.path().join("migrations");
  std::fs::create_dir_all(&migrations_dir)?;

  let sql = "CREATE TABLE \"conversations\" (\n    \"id\" TEXT PRIMARY KEY\n);";
  std::fs::write(migrations_dir.join("0001_initial.sql"), sql)?;

  let conn = create_test_connection().await?;

  let count = toolu_orm_cli::migrate::run_migrate(
    &conn,
    migrations_dir.to_str().ok_or("invalid path")?,
    Dialect::Sqlite,
  )
  .await?;
  assert_eq!(count, 1);

  let rows = conn
    .query_map::<ScalarI64>("SELECT count(*) AS v FROM conversations", vec![])
    .await?;
  assert_eq!(
    rows.first().ok_or("expected scalar row")?.v,
    0,
    "conversations table should exist and be empty"
  );

  let mig = conn
    .query_map::<MigrationNameRow>("SELECT name FROM _migrations", vec![])
    .await?;
  assert_eq!(mig.len(), 1);
  assert_eq!(
    mig.first().ok_or("expected migration row")?.name,
    "0001_initial.sql"
  );
  Ok(())
}

#[tokio::test]
async fn test_migrate_skips_already_applied() -> TestResult {
  let dir = tempfile::tempdir()?;
  let migrations_dir = dir.path().join("migrations");
  std::fs::create_dir_all(&migrations_dir)?;

  let sql = "CREATE TABLE \"conversations\" (\n    \"id\" TEXT PRIMARY KEY\n);";
  std::fs::write(migrations_dir.join("0001_initial.sql"), sql)?;

  let conn = create_test_connection().await?;

  let count = toolu_orm_cli::migrate::run_migrate(
    &conn,
    migrations_dir.to_str().ok_or("invalid path")?,
    Dialect::Sqlite,
  )
  .await?;
  assert_eq!(count, 1);

  let count = toolu_orm_cli::migrate::run_migrate(
    &conn,
    migrations_dir.to_str().ok_or("invalid path")?,
    Dialect::Sqlite,
  )
  .await?;
  assert_eq!(count, 0);
  Ok(())
}

#[tokio::test]
async fn test_migrate_with_journal_and_breakpoints() -> TestResult {
  let dir = tempfile::tempdir()?;
  let migrations_dir = dir.path().join("migrations");
  std::fs::create_dir_all(&migrations_dir)?;

  let sql = "CREATE TABLE a (id TEXT);\n--> statement-breakpoint\nCREATE TABLE b (id TEXT);";
  std::fs::write(migrations_dir.join("0001_init.sql"), sql)?;

  let mut journal = Journal::empty();
  journal.add_entry("0001_init.sql", &compute_hash(sql));
  let journal_path = migrations_dir.join("_journal.json");
  journal.write_to_path(journal_path.to_str().ok_or("invalid path")?)?;

  let conn = create_test_connection().await?;

  let count = toolu_orm_cli::migrate::run_migrate(
    &conn,
    migrations_dir.to_str().ok_or("invalid path")?,
    Dialect::Sqlite,
  )
  .await?;
  assert_eq!(count, 1);

  let a = conn
    .query_map::<ScalarI64>("SELECT count(*) AS v FROM a", vec![])
    .await?;
  assert_eq!(a.first().ok_or("table a should exist")?.v, 0);
  let b = conn
    .query_map::<ScalarI64>("SELECT count(*) AS v FROM b", vec![])
    .await?;
  assert_eq!(b.first().ok_or("table b should exist")?.v, 0);
  Ok(())
}

#[tokio::test]
async fn test_migrate_hash_mismatch_errors() -> TestResult {
  let dir = tempfile::tempdir()?;
  let migrations_dir = dir.path().join("migrations");
  std::fs::create_dir_all(&migrations_dir)?;

  std::fs::write(
    migrations_dir.join("0001_init.sql"),
    "CREATE TABLE a (id TEXT);",
  )?;

  let mut journal = Journal::empty();
  journal.add_entry("0001_init.sql", "sha256:wrong_hash");
  let journal_path = migrations_dir.join("_journal.json");
  journal.write_to_path(journal_path.to_str().ok_or("invalid path")?)?;

  let conn = create_test_connection().await?;

  let result = toolu_orm_cli::migrate::run_migrate(
    &conn,
    migrations_dir.to_str().ok_or("invalid path")?,
    Dialect::Sqlite,
  )
  .await;
  let err = result.err().ok_or("expected migration to fail")?;
  assert!(err.to_string().contains("modified"));
  Ok(())
}

#[tokio::test]
async fn ensure_migrations_table_creates_correct_schema_for_dialect() -> TestResult {
  let conn = create_test_connection().await?;

  toolu_orm_cli::migrate::ensure_migrations_table(&conn, Dialect::Sqlite).await?;

  conn
    .execute_sql(
      "INSERT INTO _migrations (name, hash) VALUES (?1, ?2)",
      vec![
        Value::Text("test.sql".to_owned()),
        Value::Text("sha256:abc".to_owned()),
      ],
    )
    .await?;

  let m = conn
    .query_map::<MigrationNameRow>("SELECT name FROM _migrations", vec![])
    .await?;
  assert_eq!(m.len(), 1);
  Ok(())
}

struct ScalarI64 {
  v: i64,
}

impl FromRow for ScalarI64 {
  const REQUIRED_COLUMNS: &'static [&'static str] = &["v"];

  fn from_pg_row(_: &tokio_postgres::Row) -> Result<Self, toolu_orm_core::error::DbCoreError> {
    Err(toolu_orm_core::error::DbCoreError::RowMapping(
      "ScalarI64 is only used in libsql migrate tests".into(),
    ))
  }

  fn from_libsql_row(row: &libsql::Row) -> Result<Self, toolu_orm_core::error::DbCoreError> {
    let v: i64 = row
      .get(0)
      .map_err(|e| toolu_orm_core::error::DbCoreError::RowMapping(e.to_string()))?;
    Ok(Self { v })
  }
}

struct MigrationNameRow {
  name: String,
}

impl FromRow for MigrationNameRow {
  const REQUIRED_COLUMNS: &'static [&'static str] = &["name"];

  fn from_pg_row(_: &tokio_postgres::Row) -> Result<Self, toolu_orm_core::error::DbCoreError> {
    Err(toolu_orm_core::error::DbCoreError::RowMapping(
      "MigrationNameRow: pg stub".into(),
    ))
  }

  fn from_libsql_row(row: &libsql::Row) -> Result<Self, toolu_orm_core::error::DbCoreError> {
    let name: String = row
      .get(0)
      .map_err(|e| toolu_orm_core::error::DbCoreError::RowMapping(e.to_string()))?;
    Ok(Self { name })
  }
}
