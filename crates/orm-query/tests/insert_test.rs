use toolu_orm_core::column::{Integer, Text};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::query_column::Column;
use toolu_orm_core::value::Value;
use toolu_orm_query::insert::InsertBuilder;

const ID: Column<Text> = Column::new("users", "id");
const EMAIL: Column<Text> = Column::new("users", "email");
const AGE: Column<Integer> = Column::new("users", "age");

const RUN_ID: Column<Text> = Column::new("run_status", "run_id");
const STATUS: Column<Text> = Column::new("run_status", "status");

const SEED_ID: Column<Text> = Column::new("seeds", "id");

// ── 1. basic_insert ───────────────────────────────────────────────────────────

#[test]
fn basic_insert() {
  let (sql, params) = InsertBuilder::new("users")
    .set(&ID, "user-1")
    .set(&EMAIL, "user@example.com")
    .to_sql_for(Dialect::Sqlite);
  assert_eq!(
    sql,
    r#"INSERT INTO "users" ("id", "email") VALUES (?1, ?2)"#
  );
  assert_eq!(
    params,
    vec![
      Value::Text("user-1".to_owned()),
      Value::Text("user@example.com".to_owned()),
    ]
  );
}

// ── 2. insert_with_null ───────────────────────────────────────────────────────

#[test]
fn insert_with_null() {
  let (sql, params) = InsertBuilder::new("users")
    .set(&ID, "user-1")
    .set_null(&EMAIL)
    .to_sql_for(Dialect::Sqlite);
  assert_eq!(
    sql,
    r#"INSERT INTO "users" ("id", "email") VALUES (?1, ?2)"#
  );
  assert_eq!(params, vec![Value::Text("user-1".to_owned()), Value::Null,]);
}

// ── 3. or_replace ─────────────────────────────────────────────────────────────

#[test]
fn or_replace() {
  let (sql, params) = InsertBuilder::new("run_status")
    .or_replace()
    .set(&RUN_ID, "run-1")
    .set(&STATUS, "running")
    .to_sql_for(Dialect::Sqlite);
  assert_eq!(
    sql,
    r#"INSERT OR REPLACE INTO "run_status" ("run_id", "status") VALUES (?1, ?2)"#
  );
  assert_eq!(
    params,
    vec![
      Value::Text("run-1".to_owned()),
      Value::Text("running".to_owned()),
    ]
  );
}

// ── 4. or_ignore ──────────────────────────────────────────────────────────────

#[test]
fn or_ignore() {
  let (sql, params) = InsertBuilder::new("seeds")
    .or_ignore()
    .set(&SEED_ID, "seed-1")
    .to_sql_for(Dialect::Sqlite);
  assert_eq!(sql, r#"INSERT OR IGNORE INTO "seeds" ("id") VALUES (?1)"#);
  assert_eq!(params, vec![Value::Text("seed-1".to_owned())]);
}

// ── 5. single_column_insert ───────────────────────────────────────────────────

#[test]
fn single_column_insert() {
  let (sql, params) = InsertBuilder::new("users")
    .set(&ID, "u1")
    .to_sql_for(Dialect::Sqlite);
  assert_eq!(sql, r#"INSERT INTO "users" ("id") VALUES (?1)"#);
  assert_eq!(params, vec![Value::Text("u1".to_owned())]);
}

// ── 6. param_order_matches_column_order ───────────────────────────────────────

#[test]
fn param_order_matches_column_order() {
  let (sql, params) = InsertBuilder::new("users")
    .set(&EMAIL, "a@b.com")
    .set(&ID, "user-2")
    .set(&AGE, 30i32)
    .to_sql_for(Dialect::Sqlite);
  assert_eq!(
    sql,
    r#"INSERT INTO "users" ("email", "id", "age") VALUES (?1, ?2, ?3)"#
  );
  assert_eq!(
    params,
    vec![
      Value::Text("a@b.com".to_owned()),
      Value::Text("user-2".to_owned()),
      Value::Integer(30),
    ]
  );
}

#[test]
fn insert_to_sql_for_postgres_uses_dollar_params() {
  let (sql, params) = InsertBuilder::new("users")
    .set(&ID, "user-1")
    .set(&EMAIL, "a@b.com")
    .to_sql_for(Dialect::Postgres);
  assert_eq!(
    sql,
    r#"INSERT INTO "users" ("id", "email") VALUES ($1, $2)"#
  );
  assert_eq!(params.len(), 2);
}

#[test]
fn insert_to_sql_for_sqlite_uses_question_params() {
  let (sql, params) = InsertBuilder::new("users")
    .set(&ID, "user-1")
    .set(&EMAIL, "a@b.com")
    .to_sql_for(Dialect::Sqlite);
  assert_eq!(
    sql,
    r#"INSERT INTO "users" ("id", "email") VALUES (?1, ?2)"#
  );
  assert_eq!(params.len(), 2);
}

#[test]
fn insert_to_sql_delegates_to_current() {
  let builder = InsertBuilder::new("users").set(&ID, "u1");
  let (via_default, _) = builder.to_sql();
  let (via_explicit, _) = builder.to_sql_for(Dialect::CURRENT);
  assert_eq!(via_default, via_explicit);
}

// ── Postgres conflict syntax ─────────────────────────────────────────────────

#[test]
fn postgres_or_ignore_emits_on_conflict_do_nothing() {
  let (sql, params) = InsertBuilder::new("seeds")
    .or_ignore()
    .set(&SEED_ID, "seed-1")
    .to_sql_for(Dialect::Postgres);
  assert_eq!(
    sql,
    r#"INSERT INTO "seeds" ("id") VALUES ($1) ON CONFLICT DO NOTHING"#
  );
  assert_eq!(params, vec![Value::Text("seed-1".to_owned())]);
}

#[test]
fn postgres_or_replace_emits_on_conflict_do_update() {
  let (sql, params) = InsertBuilder::new("run_status")
    .or_replace()
    .set(&RUN_ID, "run-1")
    .set(&STATUS, "running")
    .to_sql_for(Dialect::Postgres);
  assert_eq!(
    sql,
    r#"INSERT INTO "run_status" ("run_id", "status") VALUES ($1, $2) ON CONFLICT ("run_id") DO UPDATE SET "status" = EXCLUDED."status""#
  );
  assert_eq!(params.len(), 2);
}

#[test]
fn postgres_or_replace_single_column_uses_first_as_conflict_target() {
  let (sql, params) = InsertBuilder::new("seeds")
    .or_replace()
    .set(&SEED_ID, "seed-1")
    .to_sql_for(Dialect::Postgres);
  assert_eq!(
    sql,
    r#"INSERT INTO "seeds" ("id") VALUES ($1) ON CONFLICT ("id") DO NOTHING"#
  );
  assert_eq!(params, vec![Value::Text("seed-1".to_owned())]);
}

#[test]
fn sqlite_or_replace_still_uses_sqlite_syntax() {
  let (sql, _) = InsertBuilder::new("run_status")
    .or_replace()
    .set(&RUN_ID, "run-1")
    .set(&STATUS, "running")
    .to_sql_for(Dialect::Sqlite);
  assert_eq!(
    sql,
    r#"INSERT OR REPLACE INTO "run_status" ("run_id", "status") VALUES (?1, ?2)"#
  );
}

#[test]
fn sqlite_or_ignore_still_uses_sqlite_syntax() {
  let (sql, _) = InsertBuilder::new("seeds")
    .or_ignore()
    .set(&SEED_ID, "seed-1")
    .to_sql_for(Dialect::Sqlite);
  assert_eq!(sql, r#"INSERT OR IGNORE INTO "seeds" ("id") VALUES (?1)"#);
}

#[test]
fn postgres_no_conflict_emits_plain_insert() {
  let (sql, params) = InsertBuilder::new("users")
    .set(&ID, "user-1")
    .set(&EMAIL, "a@b.com")
    .to_sql_for(Dialect::Postgres);
  assert_eq!(
    sql,
    r#"INSERT INTO "users" ("id", "email") VALUES ($1, $2)"#
  );
  assert_eq!(params.len(), 2);
}

#[test]
fn postgres_or_replace_with_explicit_conflict_columns() {
  let (sql, _) = InsertBuilder::new("users")
    .or_replace()
    .conflict_columns(&["id"])
    .set(&ID, "user-1")
    .set(&EMAIL, "new@b.com")
    .set(&AGE, 30i32)
    .to_sql_for(Dialect::Postgres);
  assert_eq!(
    sql,
    r#"INSERT INTO "users" ("id", "email", "age") VALUES ($1, $2, $3) ON CONFLICT ("id") DO UPDATE SET "email" = EXCLUDED."email", "age" = EXCLUDED."age""#
  );
}
