//! Edge and failure modes of `run_migrate` on in-memory libsql: rollback after a failing
//! statement (journal and legacy modes), unreadable inputs, malformed journal,
//! absent directory, comment-only chunks. Assertions go through raw libsql so the file compiles in
//! every lane that has libsql, whatever `FromRow` shape orm-core exposes.

use toolu_orm_cli::migrate::{run_migrate, MigrateError};
use toolu_orm_connection::{Database, LibsqlConnection};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::journal::{compute_hash, Journal};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const BAD_JOURNAL_SQL: &str =
  "CREATE TABLE a (id TEXT);\n--> statement-breakpoint\nINSERT INTO nope VALUES (1);";
const BAD_LEGACY_SQL: &str = "CREATE TABLE a (id TEXT);\nINSERT INTO nope VALUES (1);";

async fn connect() -> Result<LibsqlConnection, Box<dyn std::error::Error>> {
  Ok(Database::init_local(":memory:").await?.connect()?)
}

fn migrations_dir() -> Result<(tempfile::TempDir, String), Box<dyn std::error::Error>> {
  let dir = tempfile::tempdir()?;
  let path = dir.path().join("migrations");
  std::fs::create_dir_all(&path)?;
  Ok((dir, path.to_str().ok_or("non-UTF8 path")?.to_owned()))
}

async fn scalar(conn: &LibsqlConnection, sql: &str) -> Result<i64, Box<dyn std::error::Error>> {
  let mut rows = conn.inner_conn().query(sql, ()).await?;
  let row = rows.next().await?.ok_or("scalar query returned no row")?;
  Ok(row.get::<i64>(0)?)
}

async fn assert_nothing_applied(conn: &LibsqlConnection) -> TestResult {
  assert_eq!(scalar(conn, "SELECT count(*) FROM _migrations").await?, 0);
  assert_eq!(
    scalar(
      conn,
      "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'a'"
    )
    .await?,
    0,
    "first statement's table survived the rollback"
  );
  Ok(())
}

#[tokio::test]
async fn journal_mode_failing_statement_rolls_back_whole_file() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  std::fs::write(format!("{dir}/0001_bad.sql"), BAD_JOURNAL_SQL)?;
  let mut journal = Journal::empty();
  journal.add_entry("0001_bad.sql", &compute_hash(BAD_JOURNAL_SQL));
  journal.write_to_path(&format!("{dir}/_journal.json"))?;
  let conn = connect().await?;

  let Err(err) = run_migrate(&conn, &dir, Dialect::Sqlite).await else {
    return Err("expected the migration to fail".into());
  };
  assert!(matches!(err, MigrateError::Database(_)), "got {err:?}");
  assert!(err.to_string().contains("0001_bad.sql"), "message: {err}");
  assert_nothing_applied(&conn).await
}

#[tokio::test]
async fn legacy_mode_failing_statement_rolls_back_whole_file() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  std::fs::write(format!("{dir}/0001_bad.sql"), BAD_LEGACY_SQL)?;
  let conn = connect().await?;

  let Err(err) = run_migrate(&conn, &dir, Dialect::Sqlite).await else {
    return Err("expected the migration to fail".into());
  };
  assert!(matches!(err, MigrateError::Database(_)), "got {err:?}");
  assert_nothing_applied(&conn).await
}

#[tokio::test]
async fn migrations_dir_that_is_a_file_is_read_dir_error() -> TestResult {
  let file = tempfile::NamedTempFile::new()?;
  let path = file.path().to_str().ok_or("non-UTF8 path")?;
  let conn = connect().await?;

  let Err(err) = run_migrate(&conn, path, Dialect::Sqlite).await else {
    return Err("expected ReadDir".into());
  };
  assert!(matches!(err, MigrateError::ReadDir(_)), "got {err:?}");
  Ok(())
}

#[tokio::test]
async fn journal_entry_without_sql_file_is_read_file_error() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let mut journal = Journal::empty();
  journal.add_entry("0001_missing.sql", "sha256:0");
  journal.write_to_path(&format!("{dir}/_journal.json"))?;
  let conn = connect().await?;

  let Err(err) = run_migrate(&conn, &dir, Dialect::Sqlite).await else {
    return Err("expected ReadFile".into());
  };
  assert!(matches!(err, MigrateError::ReadFile(_)), "got {err:?}");
  assert!(
    err.to_string().contains("0001_missing.sql"),
    "message: {err}"
  );
  Ok(())
}

#[tokio::test]
async fn malformed_journal_is_read_file_error() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  std::fs::write(format!("{dir}/_journal.json"), "{not json")?;
  let conn = connect().await?;

  let Err(err) = run_migrate(&conn, &dir, Dialect::Sqlite).await else {
    return Err("expected ReadFile".into());
  };
  assert!(matches!(err, MigrateError::ReadFile(_)), "got {err:?}");
  assert!(err.to_string().contains("_journal.json"), "message: {err}");
  Ok(())
}

#[tokio::test]
async fn absent_migrations_dir_applies_nothing() -> TestResult {
  let tmp = tempfile::tempdir()?;
  let missing = tmp.path().join("does-not-exist");
  let conn = connect().await?;
  let applied = run_migrate(&conn, missing.to_str().ok_or("path")?, Dialect::Sqlite).await?;
  assert_eq!(applied, 0);
  assert_eq!(scalar(&conn, "SELECT count(*) FROM _migrations").await?, 0);
  Ok(())
}

#[tokio::test]
async fn comment_only_breakpoint_chunk_is_skipped() -> TestResult {
  // The generator emits comment-only chunks for operations a dialect cannot
  // express (e.g. adding a CHECK on SQLite); they must not reach the driver.
  let sql = "CREATE TABLE a (id TEXT);\n--> statement-breakpoint\n\
             -- ADD CHECK: SQLite may require table rebuild\n--> statement-breakpoint\n\
             CREATE TABLE b (id TEXT);";
  let (_tmp, dir) = migrations_dir()?;
  std::fs::write(format!("{dir}/0001_with_note.sql"), sql)?;
  let mut journal = Journal::empty();
  journal.add_entry("0001_with_note.sql", &compute_hash(sql));
  journal.write_to_path(&format!("{dir}/_journal.json"))?;
  let conn = connect().await?;

  assert_eq!(run_migrate(&conn, &dir, Dialect::Sqlite).await?, 1);
  assert_eq!(
    scalar(
      &conn,
      "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name IN ('a', 'b')"
    )
    .await?,
    2
  );
  assert_eq!(scalar(&conn, "SELECT count(*) FROM _migrations").await?, 1);
  Ok(())
}
