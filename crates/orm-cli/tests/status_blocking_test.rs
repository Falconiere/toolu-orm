//! Blocking status APIs against in-memory rusqlite with no tokio runtime.

#[path = "fixtures/baseline_dir.rs"]
pub mod baseline_dir;
#[path = "fixtures/embedded_list.rs"]
pub mod embedded_list;

use toolu_orm_cli::migrate::run_migrate_blocking;
use toolu_orm_cli::status::{get_status_blocking, get_status_embedded_blocking};
use toolu_orm_connection::RusqliteConnection;
use toolu_orm_core::dialect::Dialect;

use baseline_dir::{migrations_dir, write_migrations, INIT_SQL, POSTS_SQL};
use embedded_list::{honest, list, CREATE_SQL};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const POSTS_EMBEDDED: &str = "CREATE TABLE posts (id TEXT PRIMARY KEY);";

fn connect() -> Result<RusqliteConnection, Box<dyn std::error::Error>> {
  Ok(RusqliteConnection::from_connection(
    rusqlite::Connection::open_in_memory()?,
  ))
}

#[test]
fn get_status_blocking_lists_pending() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  write_migrations(
    &dir,
    &[("0001_init.sql", INIT_SQL), ("0002_posts.sql", POSTS_SQL)],
  )?;
  let conn = connect()?;

  let before = get_status_blocking(&conn, &dir, Dialect::Sqlite)?;
  assert!(before.applied.is_empty());
  assert_eq!(
    before.pending,
    vec!["0001_init.sql".to_owned(), "0002_posts.sql".to_owned()]
  );

  assert_eq!(run_migrate_blocking(&conn, &dir, Dialect::Sqlite)?, 2);
  let after = get_status_blocking(&conn, &dir, Dialect::Sqlite)?;
  assert_eq!(
    after.applied,
    vec!["0001_init.sql".to_owned(), "0002_posts.sql".to_owned()]
  );
  assert!(after.pending.is_empty());
  Ok(())
}

#[test]
fn get_status_embedded_blocking_uses_list_order() -> TestResult {
  let conn = connect()?;
  let owned = vec![
    honest("0001_init.sql", CREATE_SQL),
    honest("0002_posts.sql", POSTS_EMBEDDED),
  ];
  let migrations = list(&owned);

  let status = get_status_embedded_blocking(&conn, &migrations, Dialect::Sqlite)?;
  assert!(status.applied.is_empty());
  assert_eq!(
    status.pending,
    vec!["0001_init.sql".to_owned(), "0002_posts.sql".to_owned()]
  );
  Ok(())
}
