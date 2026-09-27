use toolu_orm_core::{
  column::{Integer, Text},
  dialect::Dialect,
  query_column::Column,
};
use toolu_orm_query::merge::{Matched, MergeBuilder, NotMatched};

const ID: Column<Integer> = Column::new("items", "id");
const LABEL: Column<Text> = Column::new("items", "label");

#[test]
fn explicit_merge_binds_rows_and_refuses_sqlite() {
  let merge = MergeBuilder::new("items")
    .columns(&[&ID, &LABEL])
    .keys(&[&ID])
    .row(vec![1.into(), "changed".into()])
    .row(vec![2.into(), "new".into()])
    .when_matched(Matched::Update)
    .when_not_matched(NotMatched::Insert);
  let (sql, params) = merge.to_sql_for(Dialect::Lance).expect("valid merge");
  assert!(sql.starts_with("MERGE INTO \"items\" AS \"toolu_target\""));
  assert!(sql.contains("SELECT ?1, ?2 UNION ALL SELECT ?3, ?4"));
  assert_eq!(
    params,
    vec![1.into(), "changed".into(), 2.into(), "new".into()]
  );
  assert!(merge.to_sql_for(Dialect::Sqlite).is_err());
}

#[cfg(feature = "rusqlite")]
#[tokio::test]
async fn sqlite_refusal_preserves_real_rows_and_on_conflict(
) -> Result<(), Box<dyn std::error::Error>> {
  use toolu_orm_connection::{Capability, DbConnection, DbError, RusqliteConnection};
  use toolu_orm_query::insert::{InsertBuilder, OnConflict};
  let conn = RusqliteConnection::from_connection(rusqlite::Connection::open_in_memory()?);
  conn
    .execute_batch(
      "CREATE TABLE items(id INTEGER PRIMARY KEY, label TEXT); INSERT INTO items VALUES(1,'old')",
    )
    .await?;
  let error = MergeBuilder::new("items")
    .execute_on(&conn)
    .await
    .expect_err("capability before shape");
  assert!(matches!(
    error,
    DbError::UnsupportedCapability {
      capability: Capability::KeyMerge,
      ..
    }
  ));
  assert_eq!(
    conn.with_raw_connection(
      |raw| raw.query_row("SELECT label FROM items", [], |r| r.get::<_, String>(0))
    )??,
    "old"
  );
  assert_eq!(
    InsertBuilder::new("items")
      .set(&ID, 1)
      .set(&LABEL, "ignored")
      .on_conflict(OnConflict::column(&ID).do_nothing())
      .execute_on(&conn)
      .await?,
    0
  );
  Ok(())
}

#[path = "fixtures/invalid_merge.rs"]
pub mod invalid_merge;
#[path = "fixtures/merge.rs"]
pub mod merge;

#[test]
fn validation_and_postgres_parameter_order() -> Result<(), Box<dyn std::error::Error>> {
  for invalid in invalid_merge::cases()? {
    assert!(invalid.to_sql_for(Dialect::Postgres).is_err());
  }
  let (sql, params) = merge::batch()
    .row(merge::row(1, "a", 10))
    .row(merge::row(2, "b", 20))
    .to_sql_for(Dialect::Postgres)?;
  assert!(sql.contains("SELECT $1, $2, $3 UNION ALL SELECT $4, $5, $6"));
  assert_eq!(
    params,
    vec![
      1.into(),
      "a".into(),
      10.into(),
      2.into(),
      "b".into(),
      20.into()
    ]
  );
  Ok(())
}

#[test]
fn qualified_identifiers_are_escaped_and_aliases_are_internal(
) -> Result<(), Box<dyn std::error::Error>> {
  use toolu_orm_core::alias::TableRef;
  let key: Column<Integer> = Column::new("odd", "k\"ey");
  let label: Column<Text> = Column::new("odd", "va\"lue");
  let (sql, _) =
    MergeBuilder::into_table(TableRef::aliased("ta\"ble", "caller").in_database("sch\"ema"))
      .columns(&[&key, &label])
      .keys(&[&key])
      .row(vec![1.into(), "a".into()])
      .when_matched(Matched::Update)
      .when_not_matched(NotMatched::Insert)
      .to_sql_for(Dialect::Postgres)?;
  assert!(sql.contains("\"sch\"\"ema\".\"ta\"\"ble\" AS \"toolu_target\""));
  assert!(sql.contains("\"toolu_target\".\"k\"\"ey\" = \"toolu_source\".\"k\"\"ey\""));
  assert!(!sql.contains("caller"));
  Ok(())
}
