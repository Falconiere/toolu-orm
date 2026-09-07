use toolu_orm_connection::{Database, DbConnection};
use toolu_orm_core::dialect::Dialect;

type TestResult = Result<(), Box<dyn std::error::Error>>;

async fn connect_memory() -> Result<toolu_orm_connection::LibsqlConnection, std::io::Error> {
  let db = Database::init_local(":memory:")
    .await
    .map_err(|e| std::io::Error::other(format!("init_local: {e}")))?;
  db.connect()
    .map_err(|e| std::io::Error::other(format!("connect: {e}")))
}

#[tokio::test]
async fn test_status_shows_applied_and_pending() -> TestResult {
  let dir = tempfile::tempdir()?;
  let mdir = dir.path().join("migrations");
  std::fs::create_dir_all(&mdir)?;

  let conn = connect_memory().await?;

  std::fs::write(
    mdir.join("0001_initial.sql"),
    "CREATE TABLE \"t1\" (\"id\" TEXT PRIMARY KEY);",
  )?;
  toolu_orm_cli::migrate::run_migrate(&conn, mdir.to_str().ok_or("invalid path")?, Dialect::Sqlite)
    .await?;

  std::fs::write(
    mdir.join("0002_add_col.sql"),
    "ALTER TABLE \"t1\" ADD COLUMN \"name\" TEXT;",
  )?;

  let status =
    toolu_orm_cli::status::get_status(&conn, mdir.to_str().ok_or("invalid path")?, Dialect::Sqlite)
      .await?;
  assert_eq!(status.applied.len(), 1);
  assert_eq!(
    status.applied.first().ok_or("expected applied entry")?,
    "0001_initial.sql"
  );
  assert_eq!(status.pending.len(), 1);
  assert_eq!(
    status.pending.first().ok_or("expected pending entry")?,
    "0002_add_col.sql"
  );
  Ok(())
}

#[tokio::test]
async fn status_reports_applied_and_pending() -> TestResult {
  let dir = tempfile::tempdir()?;
  let migrations_dir = dir.path().to_str().ok_or("invalid path")?;

  std::fs::write(
    dir.path().join("0001_init.sql"),
    "CREATE TABLE a (id TEXT);",
  )?;
  std::fs::write(dir.path().join("0002_add.sql"), "CREATE TABLE b (id TEXT);")?;

  let conn = connect_memory().await?;

  toolu_orm_cli::migrate::ensure_migrations_table(&conn, Dialect::Sqlite).await?;
  conn
    .execute_batch("CREATE TABLE a (id TEXT);")
    .await
    .map_err(|e| format!("{e}"))?;
  toolu_orm_cli::migrate::record_migration(&conn, "0001_init.sql", "", Dialect::Sqlite).await?;

  let status = toolu_orm_cli::status::get_status(&conn, migrations_dir, Dialect::Sqlite).await?;
  assert_eq!(status.applied.len(), 1);
  assert_eq!(
    status.applied.first().ok_or("expected applied entry")?,
    "0001_init.sql"
  );
  assert_eq!(status.pending.len(), 1);
  assert_eq!(
    status.pending.first().ok_or("expected pending entry")?,
    "0002_add.sql"
  );
  Ok(())
}
