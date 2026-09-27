use super::{
  merge::TestResult,
  support::{open, seed, snapshot},
};
use toolu_orm_connection::DbConnection;
#[tokio::test]
async fn raw_repeated_sources_show_why_preflight_is_required() -> TestResult {
  let dir = tempfile::tempdir()?;
  let conn = open(dir.path())?;
  seed(&conn).await?;
  let sql = "MERGE INTO items AS t USING (VALUES (?1, ?2, ?3), (?4, ?5, ?6)) AS s(id,label,score) ON t.id=s.id WHEN MATCHED THEN UPDATE SET label=s.label,score=s.score WHEN NOT MATCHED THEN INSERT(id,label,score) VALUES(s.id,s.label,s.score)";
  assert_eq!(
    conn
      .execute_sql(
        sql,
        vec![
          1.into(),
          "first".into(),
          1.into(),
          1.into(),
          "second".into(),
          2.into()
        ]
      )
      .await?,
    2
  );
  let matched = snapshot(dir.path())?;
  assert!(
    matched
      == vec![
        (1, Some("first".into()), 1),
        (3, Some("untouched".into()), 30)
      ]
      || matched
        == vec![
          (1, Some("second".into()), 2),
          (3, Some("untouched".into()), 30)
        ]
  );
  assert_eq!(
    conn
      .execute_sql(
        sql,
        vec![
          9.into(),
          "first".into(),
          1.into(),
          9.into(),
          "second".into(),
          2.into()
        ]
      )
      .await?,
    2
  );
  drop(conn);
  let mut expected = matched;
  expected.extend([(9, Some("first".into()), 1), (9, Some("second".into()), 2)]);
  assert_eq!(snapshot(dir.path())?, expected);
  Ok(())
}
