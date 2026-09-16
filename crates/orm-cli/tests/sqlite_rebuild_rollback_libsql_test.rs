//! A rebuild that cannot finish leaves the database and the connection alone.
//!
//! Both failure paths matter: the copy hitting real data it cannot satisfy, and
//! `PRAGMA foreign_key_check` finding a row the migration orphaned while
//! foreign keys were suspended.

#[path = "fixtures/libsql_probe.rs"]
pub mod libsql_probe;
#[path = "fixtures/rebuild_registry.rs"]
pub mod rebuild_registry;

use toolu_orm_cli::generate::run_generate;
use toolu_orm_cli::migrate::{run_migrate, run_migrate_embedded, EmbeddedMigration, MigrateError};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::journal::compute_hash;

use libsql_probe::{
  column_names, connect, exec, foreign_keys_on, scalar, seed_user_and_post, text,
};
use rebuild_registry::{migrations_dir, registry_v1, registry_v2};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Deletes the parent row with foreign keys suspended, orphaning its post.
const ORPHANING_SQL: &str = "PRAGMA foreign_keys = OFF;\n\
                             --> statement-breakpoint\n\
                             DELETE FROM users WHERE id = 'u1';";

/// A hand-written rebuild in a journal-free directory, which the runner applies
/// as one batch. Its pragma still has to be hoisted out of the transaction.
const LEGACY_REBUILD_SQL: &str = "PRAGMA foreign_keys = OFF;\n\
   CREATE TABLE \"_toolu_new_users\" (\n\
     \"id\" TEXT NOT NULL PRIMARY KEY,\n\
     \"name\" TEXT NOT NULL,\n\
     \"email\" TEXT NOT NULL,\n\
     \"bio\" TEXT\n\
   );\n\
   INSERT INTO \"_toolu_new_users\" (\"id\", \"name\", \"email\", \"bio\") \
     SELECT \"id\", \"name\", \"email\", \"bio\" FROM \"users\";\n\
   DROP TABLE \"users\";\n\
   PRAGMA legacy_alter_table = ON;\n\
   ALTER TABLE \"_toolu_new_users\" RENAME TO \"users\";\n\
   PRAGMA legacy_alter_table = OFF;";

#[tokio::test]
async fn a_copy_that_violates_not_null_rolls_the_whole_rebuild_back() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = connect().await?;
  exec(&conn, "PRAGMA foreign_keys = ON").await?;

  run_generate(&registry_v1(), &dir, "init", Dialect::Sqlite)?;
  run_migrate(&conn, &dir, Dialect::Sqlite).await?;
  // `name` is still nullable here, so the v2 rebuild cannot copy this row.
  exec(
    &conn,
    "INSERT INTO users (id, email, bio) VALUES ('u1', 'ann@x.io', 'hi')",
  )
  .await?;
  run_generate(&registry_v2(), &dir, "tighten", Dialect::Sqlite)?;

  let failed = run_migrate(&conn, &dir, Dialect::Sqlite).await;
  assert!(failed.is_err(), "the impossible copy reported success");

  assert_eq!(scalar(&conn, "SELECT count(*) FROM users").await?, 1);
  assert_eq!(
    column_names(&conn, "users").await?,
    ["id", "name", "email", "bio"],
    "the original table did not come back unchanged"
  );
  assert_eq!(
    scalar(
      &conn,
      "SELECT count(*) FROM sqlite_master WHERE name LIKE '_toolu_new_%'"
    )
    .await?,
    0,
    "the staging table outlived the rolled-back migration"
  );
  assert_eq!(
    scalar(&conn, "SELECT count(*) FROM _migrations").await?,
    1,
    "the failed migration was recorded as applied"
  );
  assert!(
    foreign_keys_on(&conn).await?,
    "foreign keys were left suspended after the failure"
  );
  Ok(())
}

#[tokio::test]
async fn an_orphaned_row_fails_the_migration_and_restores_foreign_keys() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = connect().await?;
  exec(&conn, "PRAGMA foreign_keys = ON").await?;

  run_generate(&registry_v1(), &dir, "init", Dialect::Sqlite)?;
  run_migrate(&conn, &dir, Dialect::Sqlite).await?;
  seed_user_and_post(&conn).await?;

  let migrations = [EmbeddedMigration {
    name: "0002_orphan.sql",
    sql: ORPHANING_SQL,
    hash: &compute_hash(ORPHANING_SQL),
  }];
  let failed = run_migrate_embedded(&conn, &migrations, Dialect::Sqlite).await;

  match failed {
    Err(MigrateError::ForeignKeyViolation { file, count }) => {
      assert_eq!(file, "0002_orphan.sql");
      assert_eq!(count, 1);
    },
    other => return Err(format!("expected a foreign key violation, got {other:?}").into()),
  }

  assert_eq!(
    text(&conn, "SELECT id FROM users WHERE id = 'u1'")
      .await?
      .as_deref(),
    Some("u1"),
    "the deletion was not rolled back"
  );
  assert_eq!(scalar(&conn, "SELECT count(*) FROM posts").await?, 1);
  assert_eq!(
    scalar(
      &conn,
      "SELECT count(*) FROM _migrations WHERE name = '0002_orphan.sql'"
    )
    .await?,
    0
  );
  assert!(
    foreign_keys_on(&conn).await?,
    "the suspended foreign keys were never restored"
  );
  Ok(())
}

#[tokio::test]
async fn a_journal_free_rebuild_is_guarded_the_same_way() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = connect().await?;
  exec(&conn, "PRAGMA foreign_keys = ON").await?;

  run_generate(&registry_v1(), &dir, "init", Dialect::Sqlite)?;
  run_migrate(&conn, &dir, Dialect::Sqlite).await?;
  seed_user_and_post(&conn).await?;
  // Drop the journal so the runner falls back to plain directory scanning.
  std::fs::remove_file(format!("{dir}/_journal.json"))?;
  std::fs::write(format!("{dir}/0002_legacy_rebuild.sql"), LEGACY_REBUILD_SQL)?;

  assert_eq!(run_migrate(&conn, &dir, Dialect::Sqlite).await?, 1);

  assert_eq!(
    scalar(&conn, "SELECT count(*) FROM posts").await?,
    1,
    "the legacy path let the rebuild cascade-delete the child row"
  );
  assert_eq!(
    column_names(&conn, "users").await?,
    ["id", "name", "email", "bio"]
  );
  assert!(
    foreign_keys_on(&conn).await?,
    "the legacy path never restored the suspended foreign keys"
  );
  Ok(())
}
