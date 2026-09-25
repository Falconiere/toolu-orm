#[path = "fixtures/lance.rs"]
pub mod lance;
#[path = "fixtures/matrix_support.rs"]
pub mod support;

use toolu_orm_core::{dialect::Dialect, expr::Scalar, query_column::CommonOps, value::Value};
use toolu_orm_query::{
  delete::DeleteBuilder,
  insert::{InsertBuilder, OnConflict},
  select::SelectBuilder,
  update::UpdateBuilder,
};

use crate::support::{Fixture, ITEM_GROUP, ITEM_ID, ITEM_NAME, ITEM_SCORE, TestResult};

type Snapshot = Vec<(i64, String, i64)>;
type SnapshotResult = Result<Snapshot, Box<dyn std::error::Error>>;

fn snapshot(fixture: &Fixture) -> SnapshotResult {
  fixture.rows(
    "SELECT id, name, score FROM items ORDER BY id",
    &[],
    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
  )
}

#[test]
fn bound_insert_update_delete_change_only_matching_rows() -> TestResult {
  let fixture = Fixture::new()?;
  let insert = InsertBuilder::new("items")
    .set(&ITEM_ID, 5_i64)
    .set(&ITEM_GROUP, 2_i64)
    .set(&ITEM_NAME, "five")
    .set(&ITEM_SCORE, 50_i64)
    .to_sql_for(Dialect::Lance);
  assert_eq!(
    insert.0,
    r#"INSERT INTO "items" ("id", "group_id", "name", "score") VALUES (?1, ?2, ?3, ?4)"#
  );
  assert_eq!(
    insert.1,
    vec![
      Value::Integer(5),
      Value::Integer(2),
      Value::Text("five".to_owned()),
      Value::Integer(50)
    ]
  );
  assert_eq!(fixture.execute(&insert.0, &insert.1)?, 1);

  let update = UpdateBuilder::new("items")
    .set(&ITEM_NAME, "FIVE")
    .filter(ITEM_ID.eq(5_i64))
    .to_sql_for(Dialect::Lance);
  assert_eq!(
    update.0,
    r#"UPDATE "items" SET "name" = ?1 WHERE "items"."id" = ?2"#
  );
  assert_eq!(
    update.1,
    vec![Value::Text("FIVE".to_owned()), Value::Integer(5)]
  );
  assert_eq!(fixture.execute(&update.0, &update.1)?, 1);
  let missing = UpdateBuilder::new("items")
    .set(&ITEM_NAME, "missing")
    .filter(ITEM_ID.eq(999_i64))
    .to_sql_for(Dialect::Lance);
  assert_eq!(fixture.execute(&missing.0, &missing.1)?, 0);
  assert_eq!(
    snapshot(&fixture)?.last(),
    Some(&(5, "FIVE".to_owned(), 50))
  );

  let delete = DeleteBuilder::new("items")
    .filter(ITEM_ID.eq(5_i64))
    .to_sql_for(Dialect::Lance);
  assert_eq!(delete.0, r#"DELETE FROM "items" WHERE "items"."id" = ?1"#);
  assert_eq!(delete.1, vec![Value::Integer(5)]);
  assert_eq!(fixture.execute(&delete.0, &delete.1)?, 1);
  let missing = DeleteBuilder::new("items")
    .filter(ITEM_ID.eq(999_i64))
    .to_sql_for(Dialect::Lance);
  assert_eq!(fixture.execute(&missing.0, &missing.1)?, 0);
  assert_eq!(snapshot(&fixture)?.len(), 4);
  Ok(())
}

#[test]
fn bound_insert_select_copies_a_seeded_row_with_source_binds() -> TestResult {
  let fixture = Fixture::new()?;
  let source = SelectBuilder::new("items")
    .columns_qualified(&[&ITEM_GROUP, &ITEM_NAME, &ITEM_SCORE])
    .column_scalar(Scalar::bind(5_i64), "id")
    .filter(ITEM_ID.eq(1_i64));
  let (sql, params) = InsertBuilder::new("items")
    .select(&[&ITEM_GROUP, &ITEM_NAME, &ITEM_SCORE, &ITEM_ID], source)
    .to_sql_for(Dialect::Lance);
  assert_eq!(
    sql,
    r#"INSERT INTO "items" ("group_id", "name", "score", "id") SELECT "items"."group_id", "items"."name", "items"."score", ?1 AS "id" FROM "items" WHERE "items"."id" = ?2"#
  );
  assert_eq!(params, vec![Value::Integer(5), Value::Integer(1)]);
  assert_eq!(fixture.execute(&sql, &params)?, 1);
  let copied = fixture.rows(
    "SELECT id, group_id, name, score FROM items WHERE id = 5",
    &[],
    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
  )?;
  assert_eq!(copied, vec![(5_i64, 1_i64, "one".to_owned(), 10_i64)]);
  Ok(())
}

#[test]
fn bound_raw_merge_control_updates_and_inserts() -> TestResult {
  let fixture = Fixture::new()?;
  let sql = "MERGE INTO items AS target USING (SELECT ?1 AS id, ?2 AS name) AS incoming \
             ON target.id = incoming.id \
             WHEN MATCHED THEN UPDATE SET name = incoming.name \
             WHEN NOT MATCHED THEN INSERT (id, group_id, name, score) \
               VALUES (incoming.id, 2, incoming.name, 0)";
  let update = [Value::Integer(2), Value::Text("merged".to_owned())];
  let count = fixture.execute(sql, &update)?;
  assert_eq!(count, 1);
  let insert = [Value::Integer(5), Value::Text("new".to_owned())];
  let count = fixture.execute(sql, &insert)?;
  assert_eq!(count, 1);
  assert_eq!(
    snapshot(&fixture)?,
    vec![
      (1, "one".to_owned(), 10),
      (2, "merged".to_owned(), 20),
      (3, "three".to_owned(), 30),
      (4, "orphan".to_owned(), 40),
      (5, "new".to_owned(), 0),
    ]
  );
  Ok(())
}

#[test]
fn on_conflict_is_rejected_before_a_write() -> TestResult {
  let fixture = Fixture::new()?;
  let before = snapshot(&fixture)?;
  let (sql, params) = InsertBuilder::new("items")
    .set(&ITEM_ID, 1_i64)
    .set(&ITEM_GROUP, 1_i64)
    .set(&ITEM_NAME, "replacement")
    .set(&ITEM_SCORE, 100_i64)
    .on_conflict(OnConflict::column(&ITEM_ID).do_nothing())
    .to_sql_for(Dialect::Lance);
  assert_eq!(
    sql,
    r#"INSERT INTO "items" ("id", "group_id", "name", "score") VALUES (?1, ?2, ?3, ?4) ON CONFLICT ("id") DO NOTHING"#
  );
  assert_eq!(
    params,
    vec![
      Value::Integer(1),
      Value::Integer(1),
      Value::Text("replacement".to_owned()),
      Value::Integer(100),
    ]
  );
  let error = fixture
    .execute(&sql, &params)
    .expect_err("Lance ON CONFLICT must fail");
  assert_eq!(
    error.to_string(),
    "Binder Error: The specified columns as conflict target are not referenced by a UNIQUE/PRIMARY KEY CONSTRAINT or INDEX"
  );
  assert_eq!(snapshot(&fixture)?, before);
  Ok(())
}

#[test]
fn ordinary_dml_returning_is_rejected_before_each_write() -> TestResult {
  let fixture = Fixture::new()?;
  let before = snapshot(&fixture)?;
  let insert = InsertBuilder::new("items")
    .set(&ITEM_ID, 5_i64)
    .set(&ITEM_GROUP, 2_i64)
    .set(&ITEM_NAME, "five")
    .set(&ITEM_SCORE, 50_i64)
    .returning(&ITEM_ID)
    .to_sql_for(Dialect::Lance);
  let update = UpdateBuilder::new("items")
    .set(&ITEM_NAME, "changed")
    .filter(ITEM_ID.eq(1_i64))
    .returning(&ITEM_ID)
    .to_sql_for(Dialect::Lance);
  let delete = DeleteBuilder::new("items")
    .filter(ITEM_ID.eq(1_i64))
    .returning(&ITEM_ID)
    .to_sql_for(Dialect::Lance);

  assert_eq!(
    insert.0,
    r#"INSERT INTO "items" ("id", "group_id", "name", "score") VALUES (?1, ?2, ?3, ?4) RETURNING "id""#
  );
  assert_eq!(
    insert.1,
    vec![
      Value::Integer(5),
      Value::Integer(2),
      Value::Text("five".to_owned()),
      Value::Integer(50)
    ]
  );
  assert_eq!(
    update.0,
    r#"UPDATE "items" SET "name" = ?1 WHERE "items"."id" = ?2 RETURNING "id""#
  );
  assert_eq!(
    update.1,
    vec![Value::Text("changed".to_owned()), Value::Integer(1)]
  );
  assert_eq!(
    delete.0,
    r#"DELETE FROM "items" WHERE "items"."id" = ?1 RETURNING "id""#
  );
  assert_eq!(delete.1, vec![Value::Integer(1)]);

  for (name, (sql, params), expected) in [
    (
      "INSERT",
      insert,
      "Not implemented Error: Lance INSERT does not support RETURNING yet",
    ),
    (
      "UPDATE",
      update,
      "Not implemented Error: Lance UPDATE does not support RETURNING",
    ),
    (
      "DELETE",
      delete,
      "Not implemented Error: Lance DELETE does not support RETURNING yet",
    ),
  ] {
    let error = fixture
      .rows(&sql, &params, |row| row.get::<_, i64>(0))
      .expect_err("Lance DML RETURNING must fail");
    assert_eq!(error.to_string(), expected);
    assert_eq!(snapshot(&fixture)?, before, "{name} changed rows");
  }
  Ok(())
}
