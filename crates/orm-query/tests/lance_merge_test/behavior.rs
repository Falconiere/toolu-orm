use super::{
  invalid_merge,
  merge::{self, batch, row, TestResult, HOSTILE},
  support::{open, seed, snapshot},
};
use toolu_orm_connection::DbConnection;
use toolu_orm_query::merge::{Matched, NotMatched};
#[tokio::test]
async fn mixed_merge_policies_and_reopen() -> TestResult {
  let dir = tempfile::tempdir()?;
  let conn = open(dir.path())?;
  seed(&conn).await?;
  assert_eq!(
    batch()
      .row(row(1, HOSTILE, 11))
      .row(row(2, "new", 20))
      .execute_on(&conn)
      .await?,
    2
  );
  assert_eq!(
    batch()
      .when_not_matched(NotMatched::DoNothing)
      .row(row(9, "absent", 90))
      .execute_on(&conn)
      .await?,
    0
  );
  assert_eq!(
    batch()
      .when_matched(Matched::DoNothing)
      .row(row(1, "ignored", 100))
      .execute_on(&conn)
      .await?,
    0
  );
  drop(conn);
  assert_eq!(
    snapshot(dir.path())?,
    vec![
      (1, Some(HOSTILE.into()), 11),
      (2, Some("new".into()), 20),
      (3, Some("untouched".into()), 30)
    ]
  );
  Ok(())
}

#[tokio::test]
async fn duplicate_targets_update_and_repeated_sources_refuse() -> TestResult {
  let dir = tempfile::tempdir()?;
  let conn = open(dir.path())?;
  seed(&conn).await?;
  conn
    .execute_batch("INSERT INTO items VALUES(1,'duplicate',15)")
    .await?;
  assert_eq!(batch().row(row(1, "same", 50)).execute_on(&conn).await?, 2);
  let before = snapshot(dir.path())?;
  for id in [1, 9] {
    let err = batch()
      .row(row(id, "first", 1))
      .row(row(id, "second", 2))
      .execute_on(&conn)
      .await
      .expect_err("duplicate source");
    assert!(err.to_string().contains("repeated source key"), "{err}");
    assert_eq!(snapshot(dir.path())?, before);
  }
  assert_eq!(
    before,
    vec![
      (1, Some("same".into()), 50),
      (1, Some("same".into()), 50),
      (3, Some("untouched".into()), 30)
    ]
  );
  Ok(())
}

#[tokio::test]
async fn failing_mixed_statement_is_atomic_and_session_recovers() -> TestResult {
  let dir = tempfile::tempdir()?;
  let conn = open(dir.path())?;
  seed(&conn).await?;
  let before = snapshot(dir.path())?;
  let error = batch()
    .row(vec![1.into(), "changed".into(), "20".into()])
    .row(vec![2.into(), "new".into(), "not-an-integer".into()])
    .execute_on(&conn)
    .await
    .expect_err("conversion fails");
  assert!(error.to_string().contains("convert"), "{error}");
  assert_eq!(snapshot(dir.path())?, before);
  assert_eq!(
    batch()
      .row(row(1, "recovered", 12))
      .row(row(2, "new", 20))
      .execute_on(&conn)
      .await?,
    2
  );
  drop(conn);
  assert_eq!(
    snapshot(dir.path())?,
    vec![
      (1, Some("recovered".into()), 12),
      (2, Some("new".into()), 20),
      (3, Some("untouched".into()), 30)
    ]
  );
  Ok(())
}

#[tokio::test]
async fn malformed_batches_preserve_persisted_rows() -> TestResult {
  let dir = tempfile::tempdir()?;
  let conn = open(dir.path())?;
  seed(&conn).await?;
  let before = snapshot(dir.path())?;
  for invalid in invalid_merge::cases()? {
    let error = invalid.execute_on(&conn).await.expect_err("preflight");
    assert!(error.to_string().contains("invalid merge:"), "{error}");
    assert_eq!(snapshot(dir.path())?, before);
  }
  Ok(())
}

#[tokio::test]
async fn composite_keys_and_null_values_preserve_other_groups() -> TestResult {
  use merge::{ID, LABEL, SCORE};
  use toolu_orm_core::value::Value;
  use toolu_orm_query::merge::MergeBuilder;
  let dir = tempfile::tempdir()?;
  let conn = open(dir.path())?;
  seed(&conn).await?;
  conn
    .execute_batch("INSERT INTO items VALUES(1,'other group',20)")
    .await?;
  let merge = MergeBuilder::new("items")
    .columns(&[&LABEL, &SCORE, &ID])
    .keys(&[&ID, &SCORE])
    .when_matched(Matched::Update)
    .when_not_matched(NotMatched::Insert)
    .row(vec![Value::Null, 10.into(), 1.into()])
    .row(vec![Value::Null, 40.into(), 1.into()]);
  assert_eq!(merge.execute_on(&conn).await?, 2);
  drop(conn);
  assert_eq!(
    snapshot(dir.path())?,
    vec![
      (1, Some("other group".into()), 20),
      (1, None, 10),
      (1, None, 40),
      (3, Some("untouched".into()), 30)
    ]
  );
  Ok(())
}
