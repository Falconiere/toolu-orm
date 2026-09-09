//! `mark_applied_embedded` / `mark_applied_through_embedded` /
//! `get_status_embedded` against in-memory libsql: baselining an adopted
//! schema from a compile-time list, status without a migrations directory,
//! unknown and duplicate names, and the first real embedded migrate after
//! adoption.
//!
//! Assertions go through raw libsql so the file compiles in every lane that has
//! libsql, whatever `FromRow` shape orm-core exposes.

#[path = "fixtures/embedded_list.rs"]
pub mod embedded_list;

use toolu_orm_cli::migrate::{
  mark_applied_embedded, mark_applied_through_embedded, run_migrate_embedded, MigrateError,
};
use toolu_orm_cli::status::get_status_embedded;
use toolu_orm_connection::{Database, DbConnection, LibsqlConnection};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::journal::compute_hash;

use embedded_list::{honest, list, OwnedMigration, CREATE_SQL};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const POSTS_SQL: &str = "CREATE TABLE posts (id TEXT PRIMARY KEY);";
const THIRD_SQL: &str = "CREATE TABLE c (id TEXT);";

/// A database already carrying `users` (a prior migration system put it there)
/// plus an embedded list whose first entry would recreate it and also create
/// `audit`.
async fn adopted_db() -> Result<LibsqlConnection, Box<dyn std::error::Error>> {
  let conn = Database::init_local(":memory:").await?.connect()?;
  conn
    .execute_batch("CREATE TABLE users (id TEXT PRIMARY KEY)")
    .await?;
  Ok(conn)
}

fn init_list() -> Vec<OwnedMigration> {
  vec![honest("0001_init.sql", CREATE_SQL)]
}

fn three_list() -> Vec<OwnedMigration> {
  vec![
    honest("0001_init.sql", CREATE_SQL),
    honest("0002_posts.sql", POSTS_SQL),
    honest("0003_c.sql", THIRD_SQL),
  ]
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
async fn baseline_records_the_list_hash_without_running_sql() -> TestResult {
  let conn = adopted_db().await?;
  let owned = init_list();
  let migrations = list(&owned);

  assert_eq!(
    mark_applied_embedded(&conn, &migrations, &["0001_init.sql"], Dialect::Sqlite).await?,
    1
  );
  assert_eq!(
    text(&conn, "SELECT name FROM _migrations").await?,
    "0001_init.sql"
  );
  assert_eq!(
    text(&conn, "SELECT hash FROM _migrations").await?,
    compute_hash(CREATE_SQL),
    "the recorded hash must come from the embedded entry"
  );
  assert_eq!(
    has_table(&conn, "audit").await?,
    0,
    "no statement from the baselined entry may run"
  );
  Ok(())
}

#[tokio::test]
async fn migrate_after_an_embedded_baseline_applies_only_the_suffix() -> TestResult {
  let conn = adopted_db().await?;
  let owned = three_list();
  let migrations = list(&owned);

  assert_eq!(
    mark_applied_embedded(&conn, &migrations, &["0001_init.sql"], Dialect::Sqlite).await?,
    1
  );
  assert_eq!(
    run_migrate_embedded(&conn, &migrations, Dialect::Sqlite).await?,
    2
  );
  assert_eq!(has_table(&conn, "posts").await?, 1);
  assert_eq!(has_table(&conn, "c").await?, 1);
  assert_eq!(has_table(&conn, "audit").await?, 0);

  let status = get_status_embedded(&conn, &migrations, Dialect::Sqlite).await?;
  assert_eq!(
    status.applied,
    ["0001_init.sql", "0002_posts.sql", "0003_c.sql"]
  );
  assert!(status.pending.is_empty(), "pending: {:?}", status.pending);
  Ok(())
}

#[tokio::test]
async fn a_name_absent_from_the_list_records_nothing() -> TestResult {
  let conn = adopted_db().await?;
  let owned = init_list();
  let migrations = list(&owned);

  let Err(err) = mark_applied_embedded(
    &conn,
    &migrations,
    &["0001_init.sql", "0009_ghost.sql"],
    Dialect::Sqlite,
  )
  .await
  else {
    return Err("mark_applied_embedded accepted a name absent from the list".into());
  };
  assert!(matches!(err, MigrateError::NotInJournal(_)), "got {err:?}");
  assert!(err.to_string().contains("0009_ghost.sql"), "{err}");
  assert_eq!(
    has_table(&conn, "_migrations").await?,
    0,
    "a rejected baseline must not even create the bookkeeping table"
  );

  let Err(through) =
    mark_applied_through_embedded(&conn, &migrations, "0009_ghost.sql", Dialect::Sqlite).await
  else {
    return Err("mark_applied_through_embedded accepted an unknown last name".into());
  };
  assert!(
    matches!(through, MigrateError::NotInJournal(_)),
    "{through:?}"
  );

  assert_eq!(
    mark_applied_embedded(&conn, &migrations, &["0001_init.sql"], Dialect::Sqlite).await?,
    1
  );
  assert_eq!(scalar(&conn, "SELECT count(*) FROM _migrations").await?, 1);
  Ok(())
}

#[tokio::test]
async fn a_repeated_name_in_the_list_is_rejected_before_anything_is_written() -> TestResult {
  let conn = adopted_db().await?;
  let owned = vec![
    honest("0001_init.sql", CREATE_SQL),
    honest("0001_init.sql", CREATE_SQL),
  ];
  let migrations = list(&owned);

  let Err(err) =
    mark_applied_embedded(&conn, &migrations, &["0001_init.sql"], Dialect::Sqlite).await
  else {
    return Err("mark_applied_embedded accepted a duplicate list".into());
  };
  assert!(
    matches!(err, MigrateError::DuplicateMigration(_)),
    "got {err:?}"
  );
  assert_eq!(has_table(&conn, "_migrations").await?, 0);

  let Err(status_err) = get_status_embedded(&conn, &migrations, Dialect::Sqlite).await else {
    return Err("get_status_embedded accepted a duplicate list".into());
  };
  assert!(
    matches!(status_err, MigrateError::DuplicateMigration(_)),
    "{status_err:?}"
  );
  Ok(())
}

#[tokio::test]
async fn repeating_an_embedded_baseline_records_nothing_new() -> TestResult {
  let conn = adopted_db().await?;
  let owned = init_list();
  let migrations = list(&owned);
  let names = ["0001_init.sql", "0001_init.sql"];

  assert_eq!(
    mark_applied_embedded(&conn, &migrations, &names, Dialect::Sqlite).await?,
    1
  );
  assert_eq!(
    mark_applied_embedded(&conn, &migrations, &names, Dialect::Sqlite).await?,
    0
  );
  assert_eq!(scalar(&conn, "SELECT count(*) FROM _migrations").await?, 1);
  Ok(())
}

#[tokio::test]
async fn an_empty_embedded_baseline_still_creates_the_migrations_table() -> TestResult {
  let conn = adopted_db().await?;
  let owned = init_list();
  let migrations = list(&owned);

  assert_eq!(
    mark_applied_embedded(&conn, &migrations, &[], Dialect::Sqlite).await?,
    0
  );
  assert_eq!(scalar(&conn, "SELECT count(*) FROM _migrations").await?, 0);

  let status = get_status_embedded(&conn, &migrations, Dialect::Sqlite).await?;
  assert!(status.applied.is_empty());
  assert_eq!(status.pending, ["0001_init.sql"]);
  Ok(())
}

#[tokio::test]
async fn mark_applied_through_embedded_baselines_the_prefix_and_leaves_the_rest_pending(
) -> TestResult {
  let conn = adopted_db().await?;
  let owned = three_list();
  let migrations = list(&owned);

  assert_eq!(
    mark_applied_through_embedded(&conn, &migrations, "0002_posts.sql", Dialect::Sqlite).await?,
    2
  );
  let status = get_status_embedded(&conn, &migrations, Dialect::Sqlite).await?;
  assert_eq!(status.applied, ["0001_init.sql", "0002_posts.sql"]);
  assert_eq!(status.pending, ["0003_c.sql"]);
  assert_eq!(has_table(&conn, "posts").await?, 0, "0002 must not run");

  assert_eq!(
    run_migrate_embedded(&conn, &migrations, Dialect::Sqlite).await?,
    1
  );
  assert_eq!(has_table(&conn, "c").await?, 1);
  Ok(())
}
