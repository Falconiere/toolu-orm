//! What the embedded runner refuses, and the hash check underneath it: a
//! tampered body, a repeated name, a failing statement, a failing semicolon
//! chunk, and `verify_hash` without a database.

use toolu_orm_cli::migrate::{EmbeddedMigration, MigrateError};
use toolu_orm_core::journal::compute_hash;

use crate::embedded_list::{honest, tampered, CREATE_SQL, FAILING_SQL, MULTI_SEMI_FAILING_SQL};
use crate::support::{connect, has_table, migrate, recorded, TestResult, POSTS_SQL};

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
