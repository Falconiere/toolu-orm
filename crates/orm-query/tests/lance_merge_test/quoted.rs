use super::{merge::TestResult, support::open};
use toolu_orm_connection::{DbConnection, LanceConnection};
use toolu_orm_core::{
  alias::TableRef,
  column::{Boolean, Integer, Text},
  query_column::Column,
  value::Value,
};
use toolu_orm_query::merge::{Matched, MergeBuilder, NotMatched};

#[tokio::test]
async fn quoted_identifiers_text_boolean_keys_and_defaults() -> TestResult {
  let dir = tempfile::tempdir()?;
  let conn = open(dir.path())?;
  conn.execute_batch("CREATE TABLE \"odd table\"(\"key text\" VARCHAR, flag BOOLEAN, \"va\"\"lue\" BIGINT, omitted VARCHAR); INSERT INTO \"odd table\" VALUES('a',true,1,'keep')").await?;
  let key: Column<Text> = Column::new("odd table", "key text");
  let flag: Column<Boolean> = Column::new("odd table", "flag");
  let value: Column<Integer> = Column::new("odd table", "va\"lue");
  let batch = MergeBuilder::into_table(TableRef::new("odd table").in_database("main"))
    .columns(&[&key, &flag, &value])
    .keys(&[&key, &flag])
    .row(vec!["a".into(), Value::Boolean(true), 2.into()])
    .row(vec!["a".into(), Value::Boolean(false), 3.into()])
    .when_matched(Matched::Update)
    .when_not_matched(NotMatched::Insert);
  assert_eq!(batch.execute_on(&conn).await?, 2);
  drop(conn);
  let ext = std::env::var_os("LANCE_EXTENSION_PATH").ok_or("LANCE_EXTENSION_PATH required")?;
  let ns = LanceConnection::open(ext)?.attach(dir.path(), "data")?;
  let mut stmt = ns
    .connection()
    .prepare("SELECT \"key text\",flag,\"va\"\"lue\",omitted FROM \"odd table\" ORDER BY flag")?;
  let rows = stmt
    .query_map([], |r| {
      Ok((
        r.get::<_, String>(0)?,
        r.get::<_, bool>(1)?,
        r.get::<_, i64>(2)?,
        r.get::<_, Option<String>>(3)?,
      ))
    })?
    .collect::<Result<Vec<_>, _>>()?;
  assert_eq!(
    rows,
    vec![
      ("a".into(), false, 3, None),
      ("a".into(), true, 2, Some("keep".into()))
    ]
  );
  Ok(())
}
