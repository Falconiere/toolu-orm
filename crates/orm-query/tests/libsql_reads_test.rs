//! Read scenarios against an in-memory libsql database (libsql-only lane):
//! `fetch_one`/`fetch_optional`/`count`/`exists` semantics, and every
//! `Expr` operator executed inside `SelectBuilder::filter(...)`.

#[path = "fixtures/libsql_db.rs"]
mod db;
#[path = "fixtures/libsql_users.rs"]
mod users;

use toolu_orm_core::expr::Expr;
use toolu_orm_core::query_column::{CommonOps, NumericOps, TextOps};
use toolu_orm_core::value::Value;
use toolu_orm_query::select::SelectBuilder;
use toolu_orm_query::QueryError;
use users::{insert_user, User, USER_AGE, USER_COLUMNS, USER_NAME};

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ── fetch_one / fetch_optional / count / exists ─────────────────────────────

#[tokio::test]
async fn fetch_one_on_empty_table_returns_not_found() -> TestResult {
  let conn = db::setup_db().await?;

  let result: Result<User, QueryError> = SelectBuilder::new("users")
    .columns_raw(&USER_COLUMNS)
    .fetch_one(&conn)
    .await;

  match result {
    Err(QueryError::NotFound { table }) => assert_eq!(table, "users"),
    other => return Err(format!("expected NotFound, got: {other:?}").into()),
  }
  Ok(())
}

#[tokio::test]
async fn fetch_optional_returns_none_then_some() -> TestResult {
  let conn = db::setup_db().await?;

  let before: Option<User> = SelectBuilder::new("users")
    .columns_raw(&USER_COLUMNS)
    .fetch_optional(&conn)
    .await?;
  assert!(before.is_none());

  insert_user(&conn, "u1", "Alice", "alice@example.com", Some(30)).await?;

  let after: Option<User> = SelectBuilder::new("users")
    .columns_raw(&USER_COLUMNS)
    .fetch_optional(&conn)
    .await?;
  assert_eq!(after.map(|u| u.name), Some("Alice".to_owned()));
  Ok(())
}

#[tokio::test]
async fn count_returns_three_after_three_inserts() -> TestResult {
  let conn = db::setup_db().await?;
  insert_user(&conn, "u1", "Alice", "alice@example.com", Some(30)).await?;
  insert_user(&conn, "u2", "Bob", "bob@example.com", Some(25)).await?;
  insert_user(&conn, "u3", "Charlie", "charlie@example.com", Some(35)).await?;

  let count = SelectBuilder::new("users").count(&conn).await?;
  assert_eq!(count, 3);
  Ok(())
}

#[tokio::test]
async fn exists_true_for_matching_filter_false_otherwise() -> TestResult {
  let conn = db::setup_db().await?;
  insert_user(&conn, "u1", "Alice", "alice@example.com", Some(30)).await?;

  let found = SelectBuilder::new("users")
    .filter(USER_NAME.eq("Alice"))
    .exists(&conn)
    .await?;
  assert!(found);

  let missing = SelectBuilder::new("users")
    .filter(USER_NAME.eq("Zed"))
    .exists(&conn)
    .await?;
  assert!(!missing);
  Ok(())
}

#[tokio::test]
async fn fetch_one_with_two_matches_returns_first_by_order() -> TestResult {
  let conn = db::setup_db().await?;
  insert_user(&conn, "u1", "Alice", "a1@example.com", Some(30)).await?;
  insert_user(&conn, "u2", "Alice", "a2@example.com", Some(20)).await?;

  let user: User = SelectBuilder::new("users")
    .columns_raw(&USER_COLUMNS)
    .filter(USER_NAME.eq("Alice"))
    .order_by(USER_AGE.asc())
    .fetch_one(&conn)
    .await?;

  assert_eq!(user.id, "u2");
  assert_eq!(user.age, Some(20));
  Ok(())
}

// ── Filter operators ─────────────────────────────────────────────────────────
// u1=Alice/30, u2=Bob/25, u3=Charlie/None, u4=Dave/40.

async fn seeded_conn() -> Result<libsql::Connection, Box<dyn std::error::Error>> {
  let conn = db::setup_db().await?;
  insert_user(&conn, "u1", "Alice", "alice@example.com", Some(30)).await?;
  insert_user(&conn, "u2", "Bob", "bob@example.com", Some(25)).await?;
  insert_user(&conn, "u3", "Charlie", "charlie@example.com", None).await?;
  insert_user(&conn, "u4", "Dave", "dave@example.com", Some(40)).await?;
  Ok(conn)
}

async fn filtered_ids(
  conn: &libsql::Connection,
  expr: Expr,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
  let rows: Vec<User> = SelectBuilder::new("users")
    .columns_raw(&USER_COLUMNS)
    .filter(expr)
    .fetch_all(conn)
    .await?;
  let mut ids: Vec<String> = rows.into_iter().map(|u| u.id).collect();
  ids.sort_unstable();
  Ok(ids)
}

#[tokio::test]
async fn eq_and_ne_match_expected_rows() -> TestResult {
  let conn = seeded_conn().await?;
  let eq = filtered_ids(&conn, USER_NAME.eq("Alice")).await?;
  assert_eq!(eq, vec!["u1"]);
  let ne = filtered_ids(&conn, USER_NAME.ne("Alice")).await?;
  assert_eq!(ne, vec!["u2", "u3", "u4"]);
  Ok(())
}

#[tokio::test]
async fn in_list_and_not_in_match_expected_rows() -> TestResult {
  let conn = seeded_conn().await?;
  let values = [Value::from("Alice"), Value::from("Bob")];
  let inl = filtered_ids(&conn, USER_NAME.in_list(&values)).await?;
  assert_eq!(inl, vec!["u1", "u2"]);
  let not_inl = filtered_ids(&conn, USER_NAME.not_in(&values)).await?;
  assert_eq!(not_inl, vec!["u3", "u4"]);
  Ok(())
}

#[tokio::test]
async fn gt_and_lt_match_expected_rows() -> TestResult {
  let conn = seeded_conn().await?;
  let gt = filtered_ids(&conn, USER_AGE.gt(30)).await?;
  assert_eq!(gt, vec!["u4"]);
  let lt = filtered_ids(&conn, USER_AGE.lt(30)).await?;
  assert_eq!(lt, vec!["u2"]);
  Ok(())
}

#[tokio::test]
async fn gte_and_lte_match_expected_rows() -> TestResult {
  let conn = seeded_conn().await?;
  let gte = filtered_ids(&conn, USER_AGE.gte(30)).await?;
  assert_eq!(gte, vec!["u1", "u4"]);
  let lte = filtered_ids(&conn, USER_AGE.lte(30)).await?;
  assert_eq!(lte, vec!["u1", "u2"]);
  Ok(())
}

#[tokio::test]
async fn is_null_and_like_match_expected_rows() -> TestResult {
  let conn = seeded_conn().await?;
  let nulls = filtered_ids(&conn, USER_AGE.is_null()).await?;
  assert_eq!(nulls, vec!["u3"]);
  let liked = filtered_ids(&conn, USER_NAME.like("A%")).await?;
  assert_eq!(liked, vec!["u1"]);
  Ok(())
}

#[tokio::test]
async fn between_matches_inclusive_range() -> TestResult {
  let conn = seeded_conn().await?;
  let got = filtered_ids(&conn, USER_AGE.between(25, 35)).await?;
  assert_eq!(got, vec!["u1", "u2"]);
  Ok(())
}

#[tokio::test]
async fn chained_filters_continue_param_offset_into_in_list() -> TestResult {
  let conn = seeded_conn().await?;
  let values = [Value::from("Alice"), Value::from("Dave")];
  let rows: Vec<User> = SelectBuilder::new("users")
    .columns_raw(&USER_COLUMNS)
    .filter(USER_AGE.gte(20))
    .filter(USER_AGE.lte(50))
    .filter(USER_NAME.in_list(&values))
    .fetch_all(&conn)
    .await?;
  let mut ids: Vec<String> = rows.into_iter().map(|u| u.id).collect();
  ids.sort_unstable();
  assert_eq!(ids, vec!["u1", "u4"]);
  Ok(())
}

#[tokio::test]
async fn and_or_combination_matches_expected_rows() -> TestResult {
  let conn = seeded_conn().await?;
  let expr = USER_NAME
    .eq("Alice")
    .and(USER_AGE.eq(30))
    .or(USER_NAME.eq("Dave"));
  let got = filtered_ids(&conn, expr).await?;
  assert_eq!(got, vec!["u1", "u4"]);
  Ok(())
}

#[tokio::test]
async fn empty_in_list_executes_without_driver_error() -> TestResult {
  let conn = seeded_conn().await?;
  let values: [Value; 0] = [];
  let got = filtered_ids(&conn, USER_NAME.in_list(&values)).await?;
  assert!(
    got.is_empty(),
    "empty IN list should match zero rows, got: {got:?}"
  );
  Ok(())
}

#[tokio::test]
async fn empty_not_in_matches_every_row() -> TestResult {
  let conn = seeded_conn().await?;
  let values: [Value; 0] = [];
  let got = filtered_ids(&conn, USER_NAME.not_in(&values)).await?;
  assert_eq!(got, vec!["u1", "u2", "u3", "u4"]);
  Ok(())
}
