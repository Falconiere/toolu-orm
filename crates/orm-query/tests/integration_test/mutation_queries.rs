//! Tests for mutations: update, delete, count, exists, transactions, dynamic filters.
//!
//! # Public API
//!
//! Integration tests for UpdateBuilder, DeleteBuilder, count/exists,
//! transaction commit/rollback, and dynamic filter patterns.

use toolu_orm_core::query_column::CommonOps;
use toolu_orm_query::transaction::TransactionExt;
use toolu_orm_query::QueryError;

use super::{integration_users, setup, IntegrationUser};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn update_via_builder() -> TestResult {
  let conn = setup().await?;

  IntegrationUser::insert()
    .set(&integration_users::id, "u1")
    .set(&integration_users::name, "Alice")
    .set(&integration_users::email, "alice@example.com")
    .set(&integration_users::age, 30i64)
    .execute(&conn)
    .await?;

  IntegrationUser::update()
    .set(&integration_users::name, "Alicia")
    .filter(integration_users::id.eq("u1"))
    .execute(&conn)
    .await?;

  let user: IntegrationUser = IntegrationUser::select_for::<IntegrationUser>()
    .filter(integration_users::id.eq("u1"))
    .fetch_one(&conn)
    .await?;

  assert_eq!(user.name, "Alicia");
  Ok(())
}

#[tokio::test]
async fn delete_via_builder() -> TestResult {
  let conn = setup().await?;

  IntegrationUser::insert()
    .set(&integration_users::id, "u1")
    .set(&integration_users::name, "Alice")
    .set(&integration_users::email, "alice@example.com")
    .set(&integration_users::age, 30i64)
    .execute(&conn)
    .await?;

  IntegrationUser::delete()
    .filter(integration_users::id.eq("u1"))
    .execute(&conn)
    .await?;

  let users: Vec<IntegrationUser> = IntegrationUser::select_for::<IntegrationUser>()
    .fetch_all(&conn)
    .await?;

  assert!(users.is_empty());
  Ok(())
}

#[tokio::test]
async fn count_and_exists() -> TestResult {
  let conn = setup().await?;

  for (id, name, email, age) in [
    ("u1", "Alice", "alice@example.com", 30i64),
    ("u2", "Bob", "bob@example.com", 25i64),
    ("u3", "Charlie", "charlie@example.com", 35i64),
  ] {
    IntegrationUser::insert()
      .set(&integration_users::id, id)
      .set(&integration_users::name, name)
      .set(&integration_users::email, email)
      .set(&integration_users::age, age)
      .execute(&conn)
      .await?;
  }

  let count = IntegrationUser::select().count(&conn).await?;
  assert_eq!(count, 3);

  let exists = IntegrationUser::select().exists(&conn).await?;
  assert!(exists);

  let no_match = IntegrationUser::select()
    .filter(integration_users::name.eq("Zara"))
    .exists(&conn)
    .await?;
  assert!(!no_match);
  Ok(())
}

#[tokio::test]
async fn transaction_commit() -> TestResult {
  let conn = setup().await?;

  conn
    .run_transaction(|tx| async move {
      IntegrationUser::insert()
        .set(&integration_users::id, "tx1")
        .set(&integration_users::name, "Tx User")
        .set(&integration_users::email, "tx@example.com")
        .set(&integration_users::age, 20i64)
        .execute(&tx)
        .await?;
      Ok(())
    })
    .await?;

  let user: IntegrationUser = IntegrationUser::select_for::<IntegrationUser>()
    .filter(integration_users::id.eq("tx1"))
    .fetch_one(&conn)
    .await?;

  assert_eq!(user.name, "Tx User");
  Ok(())
}

#[tokio::test]
async fn transaction_rollback() -> TestResult {
  let conn = setup().await?;

  let result: Result<(), QueryError> = conn
    .run_transaction(|tx| async move {
      IntegrationUser::insert()
        .set(&integration_users::id, "tx2")
        .set(&integration_users::name, "Should Not Exist")
        .set(&integration_users::email, "nope@example.com")
        .set(&integration_users::age, 99i64)
        .execute(&tx)
        .await?;
      Err(QueryError::NotFound {
        table: "forced_rollback".to_owned(),
      })
    })
    .await;

  assert!(result.is_err());

  let users: Vec<IntegrationUser> = IntegrationUser::select_for::<IntegrationUser>()
    .fetch_all(&conn)
    .await?;
  assert!(users.is_empty());
  Ok(())
}

#[tokio::test]
async fn dynamic_filters() -> TestResult {
  let conn = setup().await?;

  for (id, name, email, age) in [
    ("u1", "Alice", "alice@example.com", 30i64),
    ("u2", "Bob", "bob@example.com", 25i64),
    ("u3", "Alice2", "alice2@example.com", 35i64),
  ] {
    IntegrationUser::insert()
      .set(&integration_users::id, id)
      .set(&integration_users::name, name)
      .set(&integration_users::email, email)
      .set(&integration_users::age, age)
      .execute(&conn)
      .await?;
  }

  // Simulate a conditional filter: only apply age filter when Some
  let age_filter: Option<i64> = Some(30);
  let mut query = IntegrationUser::select_for::<IntegrationUser>();
  if let Some(age) = age_filter {
    query = query.filter(integration_users::age.eq(age));
  }
  let users: Vec<IntegrationUser> = query.fetch_all(&conn).await?;
  assert_eq!(users.len(), 1);
  assert_eq!(users.first().map(|u| u.name.as_str()), Some("Alice"));

  // No filter applied -- returns all
  let no_filter: Option<i64> = None;
  let mut query2 = IntegrationUser::select_for::<IntegrationUser>();
  if let Some(age) = no_filter {
    query2 = query2.filter(integration_users::age.eq(age));
  }
  let all_users: Vec<IntegrationUser> = query2.fetch_all(&conn).await?;
  assert_eq!(all_users.len(), 3);
  Ok(())
}
