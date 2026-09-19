//! Applying an embedded list: every statement of every migration, re-running,
//! the empty list, a semicolon-only chunk, and slice order beating name order.

use toolu_orm_cli::migrate::run_migrate_embedded;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::journal::compute_hash;

use crate::embedded_list::{honest, CREATE_SQL, MAKE_T_SQL, MULTI_SEMI_SQL, SEED_T_SQL};
use crate::support::{
  connect, has_table, migrate, recorded, scalar, two_migrations, TestResult, THIRD_SQL,
};

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
async fn an_empty_list_still_creates_the_migrations_table() -> TestResult {
  let conn = connect().await?;

  assert_eq!(run_migrate_embedded(&conn, &[], Dialect::Sqlite).await?, 0);
  assert_eq!(scalar(&conn, "SELECT count(*) FROM _migrations").await?, 0);
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
