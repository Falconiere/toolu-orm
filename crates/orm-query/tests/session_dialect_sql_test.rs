use toolu_orm_core::column::{Integer, Text};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::query_column::{Column, CommonOps, NumericOps};
use toolu_orm_core::value::Value;
use toolu_orm_query::delete::DeleteBuilder;
use toolu_orm_query::insert::{InsertBuilder, OnConflict};
use toolu_orm_query::select::SelectBuilder;
use toolu_orm_query::update::UpdateBuilder;

const ID: Column<Integer> = Column::new("items", "id");
const NAME: Column<Text> = Column::new("items", "name");
const SCORE: Column<Integer> = Column::new("items", "score");
const QUOTED_NAME: Column<Text> = Column::new("it\"ems", "na\"me");

#[test]
fn one_builder_renders_for_three_selected_dialects() {
  let builder = SelectBuilder::new("items")
    .columns_qualified(&[&ID])
    .column_as(&NAME, "na\"me")
    .filter(SCORE.gt(15_i64))
    .limit(2)
    .offset(1);
  let expected_values = vec![Value::Integer(15), Value::Integer(2), Value::Integer(1)];
  for (dialect, tokens) in [
    (Dialect::Sqlite, ["?1", "?2", "?3"]),
    (Dialect::Postgres, ["$1", "$2", "$3"]),
    (Dialect::Lance, ["?1", "?2", "?3"]),
  ] {
    let (sql, values) = builder.to_sql_for(dialect);
    assert_eq!(
      sql,
      format!(
        "SELECT \"items\".\"id\", \"items\".\"name\" AS \"na\"\"me\" FROM \"items\" WHERE \"items\".\"score\" > {} LIMIT {} OFFSET {}",
        tokens[0], tokens[1], tokens[2]
      )
    );
    assert_eq!(values, expected_values);
  }
}

#[test]
fn lance_offset_without_limit_does_not_use_sqlite_workaround() {
  let builder = SelectBuilder::new("items")
    .columns_qualified(&[&ID])
    .offset(1);
  assert_eq!(
    builder.to_sql_for(Dialect::Lance),
    (
      "SELECT \"items\".\"id\" FROM \"items\" OFFSET ?1".to_owned(),
      vec![Value::Integer(1)]
    )
  );
  assert_eq!(
    builder.to_sql_for(Dialect::Sqlite).0,
    "SELECT \"items\".\"id\" FROM \"items\" LIMIT -1 OFFSET ?1"
  );
  assert_eq!(
    SelectBuilder::new("items")
      .columns_qualified(&[&ID])
      .to_sql_for(Dialect::Lance)
      .1,
    Vec::<Value>::new()
  );
}

#[test]
fn lance_writes_keep_bind_order_and_unsupported_clauses_explicit() {
  let insert = InsertBuilder::new("items")
    .set(&ID, 5_i64)
    .set(&NAME, "five");
  assert_eq!(
    insert.to_sql_for(Dialect::Lance),
    (
      "INSERT INTO \"items\" (\"id\", \"name\") VALUES (?1, ?2)".to_owned(),
      vec![Value::Integer(5), Value::Text("five".to_owned())]
    )
  );
  assert_eq!(
    UpdateBuilder::new("items")
      .set(&NAME, "FIVE")
      .filter(ID.eq(5_i64))
      .to_sql_for(Dialect::Lance),
    (
      "UPDATE \"items\" SET \"name\" = ?1 WHERE \"items\".\"id\" = ?2".to_owned(),
      vec![Value::Text("FIVE".to_owned()), Value::Integer(5)]
    )
  );
  assert_eq!(
    DeleteBuilder::new("items")
      .filter(ID.eq(5_i64))
      .to_sql_for(Dialect::Lance),
    (
      "DELETE FROM \"items\" WHERE \"items\".\"id\" = ?1".to_owned(),
      vec![Value::Integer(5)]
    )
  );

  let (conflict_sql, conflict_values) = InsertBuilder::new("items")
    .set(&ID, 1_i64)
    .on_conflict(OnConflict::column(&ID))
    .to_sql_for(Dialect::Lance);
  assert_eq!(
    conflict_sql,
    "INSERT INTO \"items\" (\"id\") VALUES (?1) ON CONFLICT (\"id\") DO NOTHING"
  );
  assert_eq!(conflict_values, vec![Value::Integer(1)]);
  let (returning_sql, returning_values) = insert.returning(&ID).to_sql_for(Dialect::Lance);
  assert_eq!(
    returning_sql,
    "INSERT INTO \"items\" (\"id\", \"name\") VALUES (?1, ?2) RETURNING \"id\""
  );
  assert_eq!(
    returning_values,
    vec![Value::Integer(5), Value::Text("five".to_owned())]
  );
}

#[test]
fn explicit_rendering_and_legacy_current_remain_distinct() {
  let builder = SelectBuilder::new("items").filter(ID.eq(1_i64));
  assert_eq!(builder.to_sql(), builder.to_sql_for(Dialect::CURRENT));
  assert!(builder.to_sql_for(Dialect::Postgres).0.contains("$1"));
  assert!(builder.to_sql_for(Dialect::Lance).0.contains("?1"));
  let expected_current = if cfg!(feature = "postgres") {
    Dialect::Postgres
  } else {
    Dialect::Sqlite
  };
  assert_eq!(Dialect::CURRENT, expected_current);
}

#[test]
fn identifier_quotes_are_escaped_in_each_builder() {
  let select = SelectBuilder::new("it\"ems")
    .columns_typed(&[&QUOTED_NAME])
    .to_sql_for(Dialect::Lance);
  assert_eq!(select.0, "SELECT \"na\"\"me\" FROM \"it\"\"ems\"");
  assert_eq!(
    SelectBuilder::new("it\"ems")
      .columns_raw(&["na\"me"])
      .to_sql_for(Dialect::Lance)
      .0,
    select.0
  );
  assert_eq!(
    InsertBuilder::new("it\"ems")
      .set(&QUOTED_NAME, "five")
      .to_sql_for(Dialect::Lance)
      .0,
    "INSERT INTO \"it\"\"ems\" (\"na\"\"me\") VALUES (?1)"
  );
  assert_eq!(
    UpdateBuilder::new("it\"ems")
      .set(&QUOTED_NAME, "FIVE")
      .to_sql_for(Dialect::Lance)
      .0,
    "UPDATE \"it\"\"ems\" SET \"na\"\"me\" = ?1"
  );
  assert_eq!(
    DeleteBuilder::new("it\"ems")
      .filter(QUOTED_NAME.eq("FIVE"))
      .to_sql_for(Dialect::Lance)
      .0,
    "DELETE FROM \"it\"\"ems\" WHERE \"it\"\"ems\".\"na\"\"me\" = ?1"
  );
}
