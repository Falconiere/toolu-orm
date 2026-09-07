//! `mark_applied` / `mark_applied_through` against in-memory libsql: baselining
//! an already-migrated database, the first real migrate after adoption, unknown
//! names, repeats, the empty list, an absent `.sql` file, and the hash check
//! surviving a baseline.
//!
//! Assertions go through raw libsql so the file compiles in every lane that has
//! libsql, whatever `FromRow` shape orm-core exposes.

#[path = "fixtures/baseline_dir.rs"]
pub mod baseline_dir;

use toolu_orm_cli::migrate::{mark_applied, mark_applied_through, run_migrate, MigrateError};
use toolu_orm_cli::status::get_status;
use toolu_orm_connection::{Database, DbConnection, LibsqlConnection};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::journal::compute_hash;

use baseline_dir::{migrations_dir, write_migrations, INIT_SQL, POSTS_SQL, THIRD_SQL};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A database already carrying `users` (a prior migration system put it there)
/// plus a migrations dir whose `0001_init.sql` would recreate it.
async fn adopted_db(dir: &str) -> Result<LibsqlConnection, Box<dyn std::error::Error>> {
  write_migrations(dir, &[("0001_init.sql", INIT_SQL)])?;
  let conn = Database::init_local(":memory:").await?.connect()?;
  conn
    .execute_batch("CREATE TABLE users (id TEXT PRIMARY KEY)")
    .await?;
  Ok(conn)
}

async fn baseline(conn: &LibsqlConnection, dir: &str, names: &[&str]) -> Result<u32, MigrateError> {
  mark_applied(conn, dir, names, Dialect::Sqlite).await
}

async fn migrate(conn: &LibsqlConnection, dir: &str) -> Result<u32, MigrateError> {
  run_migrate(conn, dir, Dialect::Sqlite).await
}

async fn scalar(conn: &LibsqlConnection, sql: &str) -> Result<i64, Box<dyn std::error::Error>> {
  let mut rows = conn.inner_conn().query(sql, ()).await?;
  let row = rows.next().await?.ok_or("scalar query returned no row")?;
  Ok(row.get::<i64>(0)?)
}

async fn text(conn: &LibsqlConnection, sql: &str) -> Result<String, Box<dyn std::error::Error>> {
  let mut rows = conn.inner_conn().query(sql, ()).await?;
  let row = rows.next().await?.ok_or("query returned no row")?;
  Ok(row.get::<String>(0)?)
}

/// 1 when the table exists, 0 when it does not.
async fn has_table(conn: &LibsqlConnection, name: &str) -> Result<i64, Box<dyn std::error::Error>> {
  scalar(
    conn,
    &format!("SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = '{name}'"),
  )
  .await
}

#[tokio::test]
async fn migrate_without_a_baseline_fails_on_an_adopted_database() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = adopted_db(&dir).await?;

  let Err(err) = migrate(&conn, &dir).await else {
    return Err("run_migrate re-applied 0001 on an existing schema".into());
  };
  assert!(matches!(err, MigrateError::Database(_)), "got {err:?}");
  assert_eq!(scalar(&conn, "SELECT count(*) FROM _migrations").await?, 0);
  Ok(())
}

#[tokio::test]
async fn baseline_records_the_journal_hash_without_running_the_file() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = adopted_db(&dir).await?;

  assert_eq!(baseline(&conn, &dir, &["0001_init.sql"]).await?, 1);
  assert_eq!(
    text(&conn, "SELECT name FROM _migrations").await?,
    "0001_init.sql"
  );
  assert_eq!(
    text(&conn, "SELECT hash FROM _migrations").await?,
    compute_hash(INIT_SQL),
    "the recorded hash must come from the journal"
  );
  assert_eq!(
    has_table(&conn, "audit").await?,
    0,
    "no statement from the baselined file may run"
  );
  Ok(())
}

#[tokio::test]
async fn migrate_after_a_baseline_applies_only_the_later_files() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = adopted_db(&dir).await?;
  assert_eq!(baseline(&conn, &dir, &["0001_init.sql"]).await?, 1);

  write_migrations(
    &dir,
    &[("0001_init.sql", INIT_SQL), ("0002_posts.sql", POSTS_SQL)],
  )?;

  assert_eq!(migrate(&conn, &dir).await?, 1);
  assert_eq!(has_table(&conn, "posts").await?, 1);
  assert_eq!(has_table(&conn, "audit").await?, 0);

  let status = get_status(&conn, &dir, Dialect::Sqlite).await?;
  assert_eq!(status.applied, ["0001_init.sql", "0002_posts.sql"]);
  assert!(status.pending.is_empty(), "pending: {:?}", status.pending);
  Ok(())
}

#[tokio::test]
async fn a_name_absent_from_the_journal_records_nothing() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = adopted_db(&dir).await?;

  let Err(err) = baseline(&conn, &dir, &["0001_init.sql", "0009_ghost.sql"]).await else {
    return Err("mark_applied accepted a name with no journal entry".into());
  };
  assert!(matches!(err, MigrateError::NotInJournal(_)), "got {err:?}");
  assert!(err.to_string().contains("0009_ghost.sql"), "{err}");
  assert_eq!(
    has_table(&conn, "_migrations").await?,
    0,
    "a rejected baseline must not even create the bookkeeping table"
  );

  let Err(through) = mark_applied_through(&conn, &dir, "0009_ghost.sql", Dialect::Sqlite).await
  else {
    return Err("mark_applied_through accepted an unknown last name".into());
  };
  assert!(
    matches!(through, MigrateError::NotInJournal(_)),
    "{through:?}"
  );

  // The valid name from the rejected call was not recorded: baselining it alone
  // still counts it as new, and leaves exactly one row.
  assert_eq!(baseline(&conn, &dir, &["0001_init.sql"]).await?, 1);
  assert_eq!(scalar(&conn, "SELECT count(*) FROM _migrations").await?, 1);
  Ok(())
}

#[tokio::test]
async fn repeating_a_baseline_records_nothing_new() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = adopted_db(&dir).await?;

  let names = ["0001_init.sql", "0001_init.sql"];
  assert_eq!(baseline(&conn, &dir, &names).await?, 1);
  assert_eq!(baseline(&conn, &dir, &names).await?, 0);
  assert_eq!(scalar(&conn, "SELECT count(*) FROM _migrations").await?, 1);
  Ok(())
}

#[tokio::test]
async fn an_empty_baseline_still_creates_the_migrations_table() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = adopted_db(&dir).await?;

  assert_eq!(baseline(&conn, &dir, &[]).await?, 0);
  assert_eq!(scalar(&conn, "SELECT count(*) FROM _migrations").await?, 0);
  let status = get_status(&conn, &dir, Dialect::Sqlite).await?;
  assert!(status.applied.is_empty());
  assert_eq!(status.pending, ["0001_init.sql"]);
  Ok(())
}

#[tokio::test]
async fn mark_applied_through_baselines_the_prefix_and_leaves_the_rest_pending() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = adopted_db(&dir).await?;
  write_migrations(
    &dir,
    &[
      ("0001_init.sql", INIT_SQL),
      ("0002_posts.sql", POSTS_SQL),
      ("0003_c.sql", THIRD_SQL),
    ],
  )?;

  assert_eq!(
    mark_applied_through(&conn, &dir, "0002_posts.sql", Dialect::Sqlite).await?,
    2
  );
  let status = get_status(&conn, &dir, Dialect::Sqlite).await?;
  assert_eq!(status.applied, ["0001_init.sql", "0002_posts.sql"]);
  assert_eq!(status.pending, ["0003_c.sql"]);
  assert_eq!(has_table(&conn, "posts").await?, 0, "0002 must not run");

  assert_eq!(migrate(&conn, &dir).await?, 1);
  assert_eq!(has_table(&conn, "c").await?, 1);
  Ok(())
}

#[tokio::test]
async fn a_pending_file_edited_after_a_baseline_still_fails_the_hash_check() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = adopted_db(&dir).await?;
  write_migrations(
    &dir,
    &[("0001_init.sql", INIT_SQL), ("0002_posts.sql", POSTS_SQL)],
  )?;
  assert_eq!(baseline(&conn, &dir, &["0001_init.sql"]).await?, 1);

  std::fs::write(
    format!("{dir}/0002_posts.sql"),
    "CREATE TABLE posts (id TEXT PRIMARY KEY, tampered TEXT);",
  )?;

  let Err(err) = migrate(&conn, &dir).await else {
    return Err("a tampered pending file was applied".into());
  };
  assert!(
    matches!(err, MigrateError::HashMismatch { ref file, .. } if file == "0002_posts.sql"),
    "got {err:?}"
  );
  Ok(())
}

#[tokio::test]
async fn a_journal_entry_with_no_file_on_disk_is_baselined() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = adopted_db(&dir).await?;
  std::fs::remove_file(format!("{dir}/0001_init.sql"))?;

  assert_eq!(baseline(&conn, &dir, &["0001_init.sql"]).await?, 1);
  Ok(())
}
