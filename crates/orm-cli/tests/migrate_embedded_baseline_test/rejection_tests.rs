//! Names a baseline must refuse: one absent from the list, and a list that
//! repeats a name — neither may write anything, not even the bookkeeping table.

use toolu_orm_cli::migrate::{mark_applied_embedded, mark_applied_through_embedded, MigrateError};
use toolu_orm_cli::status::get_status_embedded;
use toolu_orm_core::dialect::Dialect;

use crate::embedded_list::{honest, list, CREATE_SQL};
use crate::support::{adopted_db, has_table, init_list, scalar, TestResult};

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
