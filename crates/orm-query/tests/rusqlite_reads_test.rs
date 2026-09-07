//! Read scenarios against an in-memory rusqlite database (rusqlite-only lane,
//! synchronous): `fetch_one`/`fetch_optional`/`count`/`exists` semantics, and
//! every `Expr` operator executed inside `SelectBuilder::filter(...)`.

#[path = "fixtures/rusqlite_db.rs"]
pub mod db;
#[path = "fixtures/rusqlite_users.rs"]
pub mod users;

use toolu_orm_core::expr::Expr;
use toolu_orm_core::query_column::{CommonOps, NumericOps, TextOps};
use toolu_orm_core::value::Value;
use toolu_orm_query::select::SelectBuilder;
use toolu_orm_query::QueryError;
use users::{insert_user, User, USER_AGE, USER_COLUMNS, USER_NAME};

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ── fetch_one / fetch_optional / count / exists ─────────────────────────────

#[test]
fn fetch_one_on_empty_table_returns_not_found() -> TestResult {
  let conn = db::setup_db()?;

  let result: Result<User, QueryError> = SelectBuilder::new("users")
    .columns_raw(&USER_COLUMNS)
    .fetch_one(&conn);

  match result {
    Err(QueryError::NotFound { table }) => assert_eq!(table, "users"),
    other => return Err(format!("expected NotFound, got: {other:?}").into()),
  }
  Ok(())
}

#[test]
fn fetch_optional_returns_none_then_some() -> TestResult {
  let conn = db::setup_db()?;

  let before: Option<User> = SelectBuilder::new("users")
    .columns_raw(&USER_COLUMNS)
    .fetch_optional(&conn)?;
  assert!(before.is_none());

  insert_user(&conn, "u1", "Alice", "alice@example.com", Some(30))?;

  let after: Option<User> = SelectBuilder::new("users")
    .columns_raw(&USER_COLUMNS)
    .fetch_optional(&conn)?;
  assert_eq!(after.map(|u| u.name), Some("Alice".to_owned()));
  Ok(())
}

#[test]
fn count_returns_three_after_three_inserts() -> TestResult {
  let conn = db::setup_db()?;
  insert_user(&conn, "u1", "Alice", "alice@example.com", Some(30))?;
  insert_user(&conn, "u2", "Bob", "bob@example.com", Some(25))?;
  insert_user(&conn, "u3", "Charlie", "charlie@example.com", Some(35))?;

  let count = SelectBuilder::new("users").count(&conn)?;
  assert_eq!(count, 3);
  Ok(())
}

#[test]
fn exists_true_for_matching_filter_false_otherwise() -> TestResult {
  let conn = db::setup_db()?;
  insert_user(&conn, "u1", "Alice", "alice@example.com", Some(30))?;

  let found = SelectBuilder::new("users")
    .filter(USER_NAME.eq("Alice"))
    .exists(&conn)?;
  assert!(found);

  let missing = SelectBuilder::new("users")
    .filter(USER_NAME.eq("Zed"))
    .exists(&conn)?;
  assert!(!missing);
  Ok(())
}

#[test]
fn fetch_one_with_two_matches_returns_first_by_order() -> TestResult {
  let conn = db::setup_db()?;
  insert_user(&conn, "u1", "Alice", "a1@example.com", Some(30))?;
  insert_user(&conn, "u2", "Alice", "a2@example.com", Some(20))?;

  let user: User = SelectBuilder::new("users")
    .columns_raw(&USER_COLUMNS)
    .filter(USER_NAME.eq("Alice"))
    .order_by(USER_AGE.asc())
    .fetch_one(&conn)?;

  assert_eq!(user.id, "u2");
  assert_eq!(user.age, Some(20));
  Ok(())
}

// ── Filter operators ─────────────────────────────────────────────────────────
// u1=Alice/30, u2=Bob/25, u3=Charlie/None, u4=Dave/40.

fn seeded_conn() -> Result<rusqlite::Connection, Box<dyn std::error::Error>> {
  let conn = db::setup_db()?;
  insert_user(&conn, "u1", "Alice", "alice@example.com", Some(30))?;
  insert_user(&conn, "u2", "Bob", "bob@example.com", Some(25))?;
  insert_user(&conn, "u3", "Charlie", "charlie@example.com", None)?;
  insert_user(&conn, "u4", "Dave", "dave@example.com", Some(40))?;
  Ok(conn)
}

fn filtered_ids(
  conn: &rusqlite::Connection,
  expr: Expr,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
  let rows: Vec<User> = SelectBuilder::new("users")
    .columns_raw(&USER_COLUMNS)
    .filter(expr)
    .fetch_all(conn)?;
  let mut ids: Vec<String> = rows.into_iter().map(|u| u.id).collect();
  ids.sort_unstable();
  Ok(ids)
}

#[test]
fn eq_and_ne_match_expected_rows() -> TestResult {
  let conn = seeded_conn()?;
  let eq = filtered_ids(&conn, USER_NAME.eq("Alice"))?;
  assert_eq!(eq, vec!["u1"]);
  let ne = filtered_ids(&conn, USER_NAME.ne("Alice"))?;
  assert_eq!(ne, vec!["u2", "u3", "u4"]);
  Ok(())
}

#[test]
fn in_list_and_not_in_match_expected_rows() -> TestResult {
  let conn = seeded_conn()?;
  let values = [Value::from("Alice"), Value::from("Bob")];
  let inl = filtered_ids(&conn, USER_NAME.in_list(&values))?;
  assert_eq!(inl, vec!["u1", "u2"]);
  let not_inl = filtered_ids(&conn, USER_NAME.not_in(&values))?;
  assert_eq!(not_inl, vec!["u3", "u4"]);
  Ok(())
}

#[test]
fn gt_and_lt_match_expected_rows() -> TestResult {
  let conn = seeded_conn()?;
  let gt = filtered_ids(&conn, USER_AGE.gt(30))?;
  assert_eq!(gt, vec!["u4"]);
  let lt = filtered_ids(&conn, USER_AGE.lt(30))?;
  assert_eq!(lt, vec!["u2"]);
  Ok(())
}

#[test]
fn gte_and_lte_match_expected_rows() -> TestResult {
  let conn = seeded_conn()?;
  let gte = filtered_ids(&conn, USER_AGE.gte(30))?;
  assert_eq!(gte, vec!["u1", "u4"]);
  let lte = filtered_ids(&conn, USER_AGE.lte(30))?;
  assert_eq!(lte, vec!["u1", "u2"]);
  Ok(())
}

#[test]
fn is_null_and_like_match_expected_rows() -> TestResult {
  let conn = seeded_conn()?;
  let nulls = filtered_ids(&conn, USER_AGE.is_null())?;
  assert_eq!(nulls, vec!["u3"]);
  let liked = filtered_ids(&conn, USER_NAME.like("A%"))?;
  assert_eq!(liked, vec!["u1"]);
  Ok(())
}

#[test]
fn between_matches_inclusive_range() -> TestResult {
  let conn = seeded_conn()?;
  let got = filtered_ids(&conn, USER_AGE.between(25, 35))?;
  assert_eq!(got, vec!["u1", "u2"]);
  Ok(())
}

#[test]
fn chained_filters_continue_param_offset_into_in_list() -> TestResult {
  let conn = seeded_conn()?;
  let values = [Value::from("Alice"), Value::from("Dave")];
  let rows: Vec<User> = SelectBuilder::new("users")
    .columns_raw(&USER_COLUMNS)
    .filter(USER_AGE.gte(20))
    .filter(USER_AGE.lte(50))
    .filter(USER_NAME.in_list(&values))
    .fetch_all(&conn)?;
  let mut ids: Vec<String> = rows.into_iter().map(|u| u.id).collect();
  ids.sort_unstable();
  assert_eq!(ids, vec!["u1", "u4"]);
  Ok(())
}

#[test]
fn and_or_combination_matches_expected_rows() -> TestResult {
  let conn = seeded_conn()?;
  let expr = USER_NAME
    .eq("Alice")
    .and(USER_AGE.eq(30))
    .or(USER_NAME.eq("Dave"));
  let got = filtered_ids(&conn, expr)?;
  assert_eq!(got, vec!["u1", "u4"]);
  Ok(())
}

#[test]
fn empty_in_list_executes_without_driver_error() -> TestResult {
  let conn = seeded_conn()?;
  let values: [Value; 0] = [];
  let got = filtered_ids(&conn, USER_NAME.in_list(&values))?;
  assert!(
    got.is_empty(),
    "empty IN list should match zero rows, got: {got:?}"
  );
  Ok(())
}

#[test]
fn empty_not_in_matches_every_row() -> TestResult {
  let conn = seeded_conn()?;
  let values: [Value; 0] = [];
  let got = filtered_ids(&conn, USER_NAME.not_in(&values))?;
  assert_eq!(
    got,
    vec!["u1", "u2", "u3", "u4"],
    "empty NOT IN list should match every row, got: {got:?}"
  );
  Ok(())
}
