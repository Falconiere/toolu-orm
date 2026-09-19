//! The directory runner and the embedded runner over the same migrations:
//! whichever ran first, the other must find nothing left to apply.

use toolu_orm_cli::migrate::run_migrate;
use toolu_orm_core::dialect::Dialect;

use crate::baseline_dir::{migrations_dir, write_migrations};
use crate::embedded_list::as_files;
use crate::support::{connect, migrate, recorded, two_migrations, TestResult};

#[tokio::test]
async fn a_directory_migrated_database_accepts_the_equivalent_embedded_list() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let migrations = two_migrations();
  write_migrations(&dir, &as_files(&migrations))?;
  let conn = connect().await?;

  assert_eq!(run_migrate(&conn, &dir, Dialect::Sqlite).await?, 2);
  let from_dir = recorded(&conn).await?;

  assert_eq!(
    migrate(&conn, &migrations).await?,
    0,
    "the same migrations from memory must look already applied"
  );
  assert_eq!(recorded(&conn).await?, from_dir);
  Ok(())
}

#[tokio::test]
async fn an_embedded_migrated_database_accepts_the_equivalent_directory() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let migrations = two_migrations();
  write_migrations(&dir, &as_files(&migrations))?;
  let conn = connect().await?;

  assert_eq!(migrate(&conn, &migrations).await?, 2);
  assert_eq!(
    run_migrate(&conn, &dir, Dialect::Sqlite).await?,
    0,
    "the same migrations from disk must look already applied"
  );
  assert_eq!(recorded(&conn).await?, ["0001_init.sql", "0002_posts.sql"]);
  Ok(())
}
