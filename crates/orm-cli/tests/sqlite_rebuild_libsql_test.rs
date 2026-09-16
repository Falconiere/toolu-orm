//! A SQLite table rebuild must not touch anything but the rebuilt table.
//!
//! Issue #85: the generated rebuild renamed the old table out of the way, which
//! repoints every child's `REFERENCES` clause, and then dropped it — firing
//! `ON DELETE CASCADE`. Everything here runs the public
//! `run_generate` → `run_migrate` loop against real in-memory libsql and asserts
//! on the database, not on the SQL text.

#[path = "fixtures/libsql_probe.rs"]
pub mod libsql_probe;
#[path = "fixtures/rebuild_registry.rs"]
pub mod rebuild_registry;

use toolu_orm_cli::generate::run_generate;
use toolu_orm_cli::migrate::run_migrate;
use toolu_orm_connection::LibsqlConnection;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::schema::SchemaRegistry;

use libsql_probe::{
  column_names, connect, exec, foreign_key_violations, foreign_keys_on, posts_fk_target, scalar,
  seed_user_and_post, text,
};
use rebuild_registry::{
  migrations_dir, registry_no_action_v1, registry_no_action_v2, registry_v1, registry_v2,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Applies `first`, seeds a user and their post, then applies `second`.
async fn seeded_rebuild(
  conn: &LibsqlConnection,
  dir: &str,
  first: &SchemaRegistry,
  second: &SchemaRegistry,
) -> TestResult {
  run_generate(first, dir, "init", Dialect::Sqlite)?;
  run_migrate(conn, dir, Dialect::Sqlite).await?;
  seed_user_and_post(conn).await?;
  run_generate(second, dir, "tighten", Dialect::Sqlite)?;
  assert_eq!(run_migrate(conn, dir, Dialect::Sqlite).await?, 1);
  Ok(())
}

#[tokio::test]
async fn rebuild_keeps_cascading_child_rows_and_their_fk_target() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = connect().await?;
  exec(&conn, "PRAGMA foreign_keys = ON").await?;

  seeded_rebuild(&conn, &dir, &registry_v1(), &registry_v2()).await?;

  assert_eq!(
    scalar(&conn, "SELECT count(*) FROM posts").await?,
    1,
    "the rebuild cascade-deleted the child row"
  );
  assert_eq!(
    posts_fk_target(&conn).await?.as_deref(),
    Some("users"),
    "the child foreign key was repointed away from users"
  );
  assert_eq!(foreign_key_violations(&conn).await?, 0);
  assert_eq!(scalar(&conn, "SELECT count(*) FROM users").await?, 1);
  assert!(foreign_keys_on(&conn).await?, "foreign keys stayed off");
  Ok(())
}

#[tokio::test]
async fn rebuild_keeps_a_non_cascading_child_row() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = connect().await?;
  exec(&conn, "PRAGMA foreign_keys = ON").await?;

  seeded_rebuild(
    &conn,
    &dir,
    &registry_no_action_v1(),
    &registry_no_action_v2(),
  )
  .await?;

  assert_eq!(scalar(&conn, "SELECT count(*) FROM posts").await?, 1);
  assert_eq!(posts_fk_target(&conn).await?.as_deref(), Some("users"));
  assert_eq!(foreign_key_violations(&conn).await?, 0);
  Ok(())
}

#[tokio::test]
async fn rebuild_restores_an_index_the_diff_never_mentioned() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = connect().await?;
  exec(&conn, "PRAGMA foreign_keys = ON").await?;

  seeded_rebuild(&conn, &dir, &registry_v1(), &registry_v2()).await?;

  assert_eq!(
    scalar(
      &conn,
      "SELECT count(*) FROM sqlite_master WHERE type = 'index' AND name = 'idx_users_email'"
    )
    .await?,
    1,
    "the unchanged unique index was lost with the old table"
  );
  let duplicate = exec(
    &conn,
    "INSERT INTO users (id, name, email) VALUES ('u2', 'Bob', 'ann@x.io')",
  )
  .await;
  assert!(
    duplicate.is_err(),
    "the restored index no longer enforces uniqueness"
  );
  Ok(())
}

#[tokio::test]
async fn a_rebuild_leaves_foreign_keys_the_way_it_found_them() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = connect().await?;
  exec(&conn, "PRAGMA foreign_keys = OFF").await?;
  assert!(!foreign_keys_on(&conn).await?);

  seeded_rebuild(&conn, &dir, &registry_v1(), &registry_v2()).await?;

  assert!(
    !foreign_keys_on(&conn).await?,
    "the migration switched foreign keys on behind the caller's back"
  );
  assert_eq!(scalar(&conn, "SELECT count(*) FROM posts").await?, 1);
  assert_eq!(posts_fk_target(&conn).await?.as_deref(), Some("users"));
  assert_eq!(column_names(&conn, "users").await?.len(), 4);
  Ok(())
}

#[tokio::test]
async fn a_view_and_a_related_trigger_survive_the_rebuild() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = connect().await?;
  exec(&conn, "PRAGMA foreign_keys = ON").await?;

  run_generate(&registry_v1(), &dir, "init", Dialect::Sqlite)?;
  run_migrate(&conn, &dir, Dialect::Sqlite).await?;
  exec(
    &conn,
    "CREATE VIEW active_users AS SELECT id, email FROM users",
  )
  .await?;
  exec(
    &conn,
    "CREATE TRIGGER trg_post_ins AFTER INSERT ON posts BEGIN \
     UPDATE users SET bio = 'touched' WHERE id = NEW.author_id; END",
  )
  .await?;
  seed_user_and_post(&conn).await?;
  run_generate(&registry_v2(), &dir, "tighten", Dialect::Sqlite)?;
  run_migrate(&conn, &dir, Dialect::Sqlite).await?;

  assert_eq!(
    scalar(&conn, "SELECT count(*) FROM active_users").await?,
    1,
    "the view over the rebuilt table stopped resolving"
  );
  exec(
    &conn,
    "INSERT INTO posts (id, author_id, title) VALUES ('p2', 'u1', 'Second')",
  )
  .await?;
  assert_eq!(
    text(&conn, "SELECT bio FROM users WHERE id = 'u1'")
      .await?
      .as_deref(),
    Some("touched"),
    "the trigger on posts no longer reaches the rebuilt users table"
  );
  Ok(())
}
