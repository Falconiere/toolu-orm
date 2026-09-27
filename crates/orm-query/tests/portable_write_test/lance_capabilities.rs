use super::support::TestResult;
use toolu_orm_connection::{Capability, DbConnection, DbError, LanceConnection, LanceDbConnection};
use toolu_orm_core::{
  column::{Integer, Text},
  dialect::Dialect,
  error::DbCoreError,
  query_column::{Column, CommonOps},
  row::{FromRow, LanceRow},
  value::Value,
};
use toolu_orm_query::{
  delete::DeleteBuilder,
  insert::{InsertBuilder, OnConflict},
  select::SelectBuilder,
  update::UpdateBuilder,
};

const ID: Column<Integer> = Column::new("items", "id");
const LABEL: Column<Text> = Column::new("items", "label");
const TEXT: &str = "RETURNING; ON CONFLICT; BEGIN -- O'Brien 東京";

#[derive(Debug, PartialEq)]
struct Item {
  id: Value,
  label: Value,
}
impl FromRow for Item {
  const REQUIRED_COLUMNS: &'static [&'static str] = &["id", "label"];
  fn from_lance_row(row: &LanceRow) -> Result<Self, DbCoreError> {
    Ok(Self {
      id: row.get("id")?.clone(),
      label: row.get("label")?.clone(),
    })
  }
}

fn open(directory: &std::path::Path) -> Result<LanceDbConnection, Box<dyn std::error::Error>> {
  let extension =
    std::env::var_os("LANCE_EXTENSION_PATH").ok_or("LANCE_EXTENSION_PATH required")?;
  Ok(LanceDbConnection::from_namespace(
    LanceConnection::open(extension)?.attach(directory, "data")?,
  ))
}

fn refused<T>(result: Result<T, DbError>, expected: Capability) -> TestResult {
  let error = result.err().ok_or("operation must refuse")?;
  assert!(
    matches!(error, DbError::UnsupportedCapability { backend: Dialect::Lance, capability } if capability == expected)
  );
  assert!(error.to_string().contains(expected.as_str()));
  assert!(error.to_string().contains(expected.alternative()));
  Ok(())
}

async fn assert_seed(conn: &LanceDbConnection) -> TestResult {
  // Empty requirements are the same guard, so supported SELECT has no false refusal.
  conn.require_capabilities(&[])?;
  let (sql, params) = SelectBuilder::new("items")
    .columns_raw(&["id", "label"])
    .filter(LABEL.eq(TEXT))
    .to_sql_for(conn.dialect());
  assert_eq!(
    conn.query_map::<Item>(&sql, params).await?,
    vec![Item {
      id: Value::Integer(1),
      label: Value::Text(TEXT.into()),
    }]
  );
  assert_eq!(
    conn
      .query_map::<Item>("SELECT id, label FROM items", vec![])
      .await?
      .len(),
    1
  );
  Ok(())
}

#[tokio::test]
async fn unsupported_mutations_preserve_rows_after_reopen() -> TestResult {
  let directory = tempfile::tempdir()?;
  let conn = open(directory.path())?;
  conn
    .execute_batch("CREATE TABLE items (id BIGINT, label VARCHAR)")
    .await?;
  InsertBuilder::new("items")
    .set(&ID, 1_i64)
    .set(&LABEL, TEXT)
    .execute_on(&conn)
    .await?;
  let candidates = [
    InsertBuilder::new("items").set(&ID, 2_i64).or_ignore(),
    InsertBuilder::new("items").set(&ID, 2_i64).or_replace(),
    InsertBuilder::new("items")
      .set(&ID, 1_i64)
      .on_conflict(OnConflict::column(&ID)),
    InsertBuilder::new("items")
      .set(&ID, 1_i64)
      .on_conflict(OnConflict::column(&ID).set(&LABEL, "changed")),
    InsertBuilder::new("items")
      .select_raw(
        &["id", "label"],
        SelectBuilder::new("items").columns_raw(&["id", "label"]),
      )
      .on_conflict(OnConflict::column(&ID)),
    InsertBuilder::new("items")
      .set(&LABEL, Value::Uuid("unsupported Lance codec".into()))
      .or_ignore(),
    InsertBuilder::new("missing_table")
      .set(&ID, 2_i64)
      .or_ignore()
      .returning(&ID),
  ];
  for builder in candidates {
    refused(builder.execute_on(&conn).await, Capability::OnConflict)?;
    assert_seed(&conn).await?;
  }
  refused(
    InsertBuilder::new("items")
      .set(&ID, 2_i64)
      .returning(&ID)
      .execute_on(&conn)
      .await,
    Capability::DmlReturning,
  )?;
  assert_seed(&conn).await?;
  for id in [1_i64, 999] {
    refused(
      UpdateBuilder::new("items")
        .set(&LABEL, "changed")
        .filter(ID.eq(id))
        .returning(&ID)
        .execute_on(&conn)
        .await,
      Capability::DmlReturning,
    )?;
    assert_seed(&conn).await?;
    refused(
      DeleteBuilder::new("items")
        .filter(ID.eq(id))
        .returning(&ID)
        .execute_on(&conn)
        .await,
      Capability::DmlReturning,
    )?;
    assert_seed(&conn).await?;
  }
  drop(conn);
  let reopened = open(directory.path())?;
  assert_seed(&reopened).await?;
  assert_eq!(
    InsertBuilder::new("items")
      .set(&ID, 2_i64)
      .set(&LABEL, TEXT)
      .conflict_columns(&["id"])
      .execute_on(&reopened)
      .await?,
    1
  );
  Ok(())
}

#[tokio::test]
async fn transaction_and_constraint_requirements_refuse_before_writes() -> TestResult {
  let directory = tempfile::tempdir()?;
  let conn = open(directory.path())?;
  conn
    .execute_batch("CREATE TABLE items (id BIGINT, label VARCHAR)")
    .await?;
  InsertBuilder::new("items")
    .set(&ID, 1_i64)
    .set(&LABEL, TEXT)
    .execute_on(&conn)
    .await?;
  refused(conn.begin().await, Capability::MultiStatementTransaction)?;
  assert_seed(&conn).await?;
  for capability in [
    Capability::PrimaryKey,
    Capability::UniqueConstraint,
    Capability::UniqueIndex,
    Capability::ForeignKey,
    Capability::NotNull,
    Capability::CheckConstraint,
  ] {
    let result = async {
      conn.require_capabilities(&[capability])?;
      conn
        .execute_batch("CREATE TABLE must_not_exist (id BIGINT)")
        .await
    }
    .await;
    refused(result, capability)?;
    assert_seed(&conn).await?;
  }
  drop(conn);
  let namespace = LanceConnection::open(
    std::env::var_os("LANCE_EXTENSION_PATH").ok_or("LANCE_EXTENSION_PATH required")?,
  )?
  .attach(directory.path(), "data")?;
  assert_eq!(namespace.list_tables()?, vec!["items"]);
  let reopened = LanceDbConnection::from_namespace(namespace);
  assert_seed(&reopened).await?;
  assert_eq!(DeleteBuilder::new("items").execute_on(&reopened).await?, 1);
  Ok(())
}
