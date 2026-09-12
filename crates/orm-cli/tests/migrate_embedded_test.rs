//! `run_migrate_embedded` against in-memory libsql: applying a compile-time
//! list, re-running it, a tampered body, a repeated name, the empty list, a
//! failing statement, slice order beating name order, the database-free hash
//! check, and interchange with the directory runner in both directions.
//!
//! Assertions go through raw libsql so the file compiles in every lane that has
//! libsql, whatever `FromRow` shape orm-core exposes.

#[path = "fixtures/baseline_dir.rs"]
pub mod baseline_dir;
#[path = "fixtures/embedded_list.rs"]
pub mod embedded_list;

use toolu_orm_cli::migrate::{run_migrate, run_migrate_embedded, EmbeddedMigration, MigrateError};
use toolu_orm_connection::{Database, LibsqlConnection};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::journal::compute_hash;

use baseline_dir::{migrations_dir, write_migrations};
use embedded_list::{
  as_files, honest, list, tampered, OwnedMigration, CREATE_SQL, FAILING_SQL, MAKE_T_SQL,
  MULTI_SEMI_FAILING_SQL, MULTI_SEMI_SQL, SEED_T_SQL,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const POSTS_SQL: &str = "CREATE TABLE posts (id TEXT PRIMARY KEY);";
const THIRD_SQL: &str = "CREATE TABLE c (id TEXT);";

async fn connect() -> Result<LibsqlConnection, Box<dyn std::error::Error>> {
  Ok(Database::init_local(":memory:").await?.connect()?)
}

async fn migrate(conn: &LibsqlConnection, owned: &[OwnedMigration]) -> Result<u32, MigrateError> {
  run_migrate_embedded(conn, &list(owned), Dialect::Sqlite).await
}

async fn scalar(conn: &LibsqlConnection, sql: &str) -> Result<i64, Box<dyn std::error::Error>> {
  let mut rows = conn.inner_conn().query(sql, ()).await?;
  let row = rows.next().await?.ok_or("scalar query returned no row")?;
  Ok(row.get::<i64>(0)?)
}

/// 1 when the table exists, 0 when it does not.
async fn has_table(conn: &LibsqlConnection, name: &str) -> Result<i64, Box<dyn std::error::Error>> {
  scalar(
    conn,
    &format!("SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = '{name}'"),
  )
  .await
}

/// The recorded names in `_migrations.id` order — the order they were applied.
async fn recorded(conn: &LibsqlConnection) -> Result<Vec<String>, Box<dyn std::error::Error>> {
  let mut rows = conn
    .inner_conn()
    .query("SELECT name FROM _migrations ORDER BY id", ())
    .await?;
  let mut names = Vec::new();
  while let Some(row) = rows.next().await? {
    names.push(row.get::<String>(0)?);
  }
  Ok(names)
}

fn two_migrations() -> Vec<OwnedMigration> {
  vec![
    honest("0001_init.sql", CREATE_SQL),
    honest("0002_posts.sql", POSTS_SQL),
  ]
}

#[tokio::test]
async fn an_embedded_list_applies_every_statement_of_every_migration() -> TestResult {
  let conn = connect().await?;

  assert_eq!(migrate(&conn, &two_migrations()).await?, 2);
  assert_eq!(has_table(&conn, "users").await?, 1);
  assert_eq!(
    has_table(&conn, "audit").await?,
    1,
    "the statement after the breakpoint must run too"
  );
  assert_eq!(has_table(&conn, "posts").await?, 1);
  assert_eq!(recorded(&conn).await?, ["0001_init.sql", "0002_posts.sql"]);
  assert_eq!(
    scalar(
      &conn,
      &format!(
        "SELECT count(*) FROM _migrations WHERE name = '0001_init.sql' AND hash = '{}'",
        compute_hash(CREATE_SQL)
      )
    )
    .await?,
    1,
    "the declared hash must be what is recorded"
  );
  Ok(())
}

#[tokio::test]
async fn re_running_applies_nothing_and_a_new_entry_applies_alone() -> TestResult {
  let conn = connect().await?;
  let mut migrations = two_migrations();
  assert_eq!(migrate(&conn, &migrations).await?, 2);

  assert_eq!(migrate(&conn, &migrations).await?, 0);

  migrations.push(honest("0003_c.sql", THIRD_SQL));
  assert_eq!(migrate(&conn, &migrations).await?, 1);
  assert_eq!(has_table(&conn, "c").await?, 1);
  assert_eq!(
    recorded(&conn).await?,
    ["0001_init.sql", "0002_posts.sql", "0003_c.sql"]
  );
  Ok(())
}

#[tokio::test]
async fn an_edited_migration_fails_the_hash_check_after_the_earlier_one_applied() -> TestResult {
  let conn = connect().await?;
  let migrations = vec![
    honest("0001_init.sql", CREATE_SQL),
    tampered(
      "0002_posts.sql",
      POSTS_SQL,
      "CREATE TABLE posts (id TEXT PRIMARY KEY, tampered TEXT);",
    ),
  ];

  let Err(err) = migrate(&conn, &migrations).await else {
    return Err("a tampered embedded migration was applied".into());
  };
  assert!(
    matches!(err, MigrateError::HashMismatch { ref file, ref expected, .. }
      if file == "0002_posts.sql" && expected == &compute_hash(POSTS_SQL)),
    "got {err:?}"
  );
  assert_eq!(has_table(&conn, "posts").await?, 0);
  assert_eq!(
    recorded(&conn).await?,
    ["0001_init.sql"],
    "the valid migration before it stays applied"
  );
  Ok(())
}

#[tokio::test]
async fn a_repeated_name_is_rejected_before_anything_is_written() -> TestResult {
  let conn = connect().await?;
  let migrations = vec![
    honest("0001_init.sql", CREATE_SQL),
    honest("0001_init.sql", POSTS_SQL),
  ];

  let Err(err) = migrate(&conn, &migrations).await else {
    return Err("a list repeating a name was accepted".into());
  };
  assert!(
    matches!(err, MigrateError::DuplicateMigration(_)),
    "got {err:?}"
  );
  assert!(err.to_string().contains("0001_init.sql"), "{err}");
  assert_eq!(
    has_table(&conn, "_migrations").await?,
    0,
    "a rejected list must not even create the bookkeeping table"
  );
  assert_eq!(has_table(&conn, "users").await?, 0);
  Ok(())
}

#[tokio::test]
async fn an_empty_list_still_creates_the_migrations_table() -> TestResult {
  let conn = connect().await?;

  assert_eq!(run_migrate_embedded(&conn, &[], Dialect::Sqlite).await?, 0);
  assert_eq!(scalar(&conn, "SELECT count(*) FROM _migrations").await?, 0);
  Ok(())
}

#[tokio::test]
async fn a_failing_statement_rolls_back_only_its_own_migration() -> TestResult {
  let conn = connect().await?;
  let migrations = vec![
    honest("0001_init.sql", CREATE_SQL),
    honest("0002_bad.sql", FAILING_SQL),
  ];

  let Err(err) = migrate(&conn, &migrations).await else {
    return Err("a failing statement did not stop the run".into());
  };
  assert!(matches!(err, MigrateError::Database(_)), "got {err:?}");
  assert!(err.to_string().contains("0002_bad.sql"), "{err}");
  assert_eq!(
    has_table(&conn, "half").await?,
    0,
    "the first statement's table survived the rollback"
  );
  assert_eq!(recorded(&conn).await?, ["0001_init.sql"]);
  Ok(())
}

#[tokio::test]
async fn a_semicolon_chunk_without_breakpoint_applies_every_statement() -> TestResult {
  let conn = connect().await?;
  let migrations = vec![honest("0001_multi.sql", MULTI_SEMI_SQL)];

  assert_eq!(migrate(&conn, &migrations).await?, 1);
  assert_eq!(has_table(&conn, "alpha").await?, 1);
  assert_eq!(has_table(&conn, "beta").await?, 1);
  assert_eq!(recorded(&conn).await?, ["0001_multi.sql"]);
  Ok(())
}

#[tokio::test]
async fn a_failing_semicolon_chunk_rolls_back_the_whole_migration() -> TestResult {
  let conn = connect().await?;
  let migrations = vec![honest("0001_bad_multi.sql", MULTI_SEMI_FAILING_SQL)];

  let Err(err) = migrate(&conn, &migrations).await else {
    return Err("a failing semicolon chunk was applied".into());
  };
  assert!(matches!(err, MigrateError::Database(_)), "got {err:?}");
  assert!(err.to_string().contains("0001_bad_multi.sql"), "{err}");
  assert_eq!(has_table(&conn, "alpha").await?, 0);
  assert_eq!(recorded(&conn).await?, [] as [&str; 0]);
  Ok(())
}

#[tokio::test]
async fn the_list_order_wins_over_the_name_order() -> TestResult {
  let conn = connect().await?;
  // Declared so that sorting by name would run the insert before the create.
  let migrations = vec![
    honest("0009_make_t.sql", MAKE_T_SQL),
    honest("0001_seed_t.sql", SEED_T_SQL),
  ];

  assert_eq!(migrate(&conn, &migrations).await?, 2);
  assert_eq!(scalar(&conn, "SELECT count(*) FROM t").await?, 1);
  assert_eq!(
    recorded(&conn).await?,
    ["0009_make_t.sql", "0001_seed_t.sql"]
  );
  Ok(())
}

#[test]
fn verify_hash_checks_a_migration_without_a_database() {
  let hash = compute_hash(CREATE_SQL);
  let good = EmbeddedMigration {
    name: "0001_init.sql",
    sql: CREATE_SQL,
    hash: &hash,
  };
  assert!(good.verify_hash().is_ok());

  let bad = EmbeddedMigration {
    name: "0001_init.sql",
    sql: POSTS_SQL,
    hash: &hash,
  };
  assert!(matches!(
    bad.verify_hash(),
    Err(MigrateError::HashMismatch { .. })
  ));
}

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
