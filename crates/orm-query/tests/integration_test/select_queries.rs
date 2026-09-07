//! Tests for SELECT queries: fetch_all, fetch_one, fetch_optional, filters, order, pagination.
//!
//! # Public API
//!
//! Integration tests for SelectBuilder with an in-memory database.

use toolu_orm_core::query_column::CommonOps;
use toolu_orm_query::QueryError;

use super::{integration_users, setup, IntegrationUser};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn insert_via_builder_and_select_back() -> TestResult {
  let conn = setup().await?;

  IntegrationUser::insert()
    .set(&integration_users::id, "u1")
    .set(&integration_users::name, "Alice")
    .set(&integration_users::email, "alice@example.com")
    .set(&integration_users::age, 30i64)
    .execute(&conn)
    .await?;

  let users: Vec<IntegrationUser> = IntegrationUser::select_for::<IntegrationUser>()
    .fetch_all(&conn)
    .await?;

  assert_eq!(users.len(), 1);
  let user = users.first().ok_or("expected at least one user")?;
  assert_eq!(user.id, "u1");
  assert_eq!(user.name, "Alice");
  assert_eq!(user.email, "alice@example.com");
  assert_eq!(user.age, 30i64);
  Ok(())
}

#[tokio::test]
async fn select_with_filter() -> TestResult {
  let conn = setup().await?;

  IntegrationUser::insert()
    .set(&integration_users::id, "u1")
    .set(&integration_users::name, "Alice")
    .set(&integration_users::email, "alice@example.com")
    .set(&integration_users::age, 30i64)
    .execute(&conn)
    .await?;

  IntegrationUser::insert()
    .set(&integration_users::id, "u2")
    .set(&integration_users::name, "Bob")
    .set(&integration_users::email, "bob@example.com")
    .set(&integration_users::age, 25i64)
    .execute(&conn)
    .await?;

  let users: Vec<IntegrationUser> = IntegrationUser::select_for::<IntegrationUser>()
    .filter(integration_users::name.eq("Alice"))
    .fetch_all(&conn)
    .await?;

  assert_eq!(users.len(), 1);
  assert_eq!(users.first().map(|u| u.name.as_str()), Some("Alice"));
  Ok(())
}

#[tokio::test]
async fn select_with_order_and_pagination() -> TestResult {
  let conn = setup().await?;

  for (id, name, email, age) in [
    ("u1", "Charlie", "charlie@example.com", 35i64),
    ("u2", "Alice", "alice@example.com", 30i64),
    ("u3", "Bob", "bob@example.com", 25i64),
  ] {
    IntegrationUser::insert()
      .set(&integration_users::id, id)
      .set(&integration_users::name, name)
      .set(&integration_users::email, email)
      .set(&integration_users::age, age)
      .execute(&conn)
      .await?;
  }

  let users: Vec<IntegrationUser> = IntegrationUser::select_for::<IntegrationUser>()
    .order_by(integration_users::name.asc())
    .limit(2)
    .fetch_all(&conn)
    .await?;

  assert_eq!(users.len(), 2);
  assert_eq!(users.first().map(|u| u.name.as_str()), Some("Alice"));
  assert_eq!(users.get(1).map(|u| u.name.as_str()), Some("Bob"));
  Ok(())
}

#[tokio::test]
async fn fetch_optional_found_and_not_found() -> TestResult {
  let conn = setup().await?;

  IntegrationUser::insert()
    .set(&integration_users::id, "u1")
    .set(&integration_users::name, "Alice")
    .set(&integration_users::email, "alice@example.com")
    .set(&integration_users::age, 30i64)
    .execute(&conn)
    .await?;

  let found: Option<IntegrationUser> = IntegrationUser::select_for::<IntegrationUser>()
    .filter(integration_users::id.eq("u1"))
    .fetch_optional(&conn)
    .await?;
  assert!(found.is_some());
  assert_eq!(found.map(|u| u.name), Some("Alice".to_owned()));

  let not_found: Option<IntegrationUser> = IntegrationUser::select_for::<IntegrationUser>()
    .filter(integration_users::id.eq("does-not-exist"))
    .fetch_optional(&conn)
    .await?;
  assert!(not_found.is_none());
  Ok(())
}

#[tokio::test]
async fn fetch_one_not_found_returns_error() -> TestResult {
  let conn = setup().await?;

  let result: Result<IntegrationUser, QueryError> =
    IntegrationUser::select_for::<IntegrationUser>()
      .filter(integration_users::id.eq("ghost"))
      .fetch_one(&conn)
      .await;

  assert!(
    matches!(result, Err(QueryError::NotFound { .. })),
    "expected NotFound, got: {result:?}"
  );
  Ok(())
}
