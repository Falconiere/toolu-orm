//! Baselining an adopted schema: what is recorded, what is deliberately not
//! run, and what the first real embedded migrate does afterwards.

use toolu_orm_cli::migrate::{
  mark_applied_embedded, mark_applied_through_embedded, run_migrate_embedded,
};
use toolu_orm_cli::status::get_status_embedded;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::journal::compute_hash;

use crate::embedded_list::{list, CREATE_SQL};
use crate::support::{adopted_db, has_table, init_list, scalar, text, three_list, TestResult};

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
