//! Executed `SelectBuilder` scenarios on a live Postgres: fetch semantics and
//! every filter operator against real rows, including parameter-number
//! continuation across three filters and nested AND/OR.
//!
//! Needs `docker compose -f docker-compose.test.yaml up -d --wait` and
//! `TEST_DB_PORT=5434` locally; compiles only via the five-crate postgres lane.

#[path = "fixtures/postgres_db.rs"]
pub mod pg;

use toolu_orm_core::expr::Expr;
use toolu_orm_core::query_column::{CommonOps, NumericOps, TextOps};
use toolu_orm_core::value::Value;
use toolu_orm_query::QueryError;

use pg::{client, insert_user, users_select, TestResult, User, AGE, EMAIL, ID, NAME};

#[tokio::test]
async fn fetch_one_on_empty_table_is_not_found() -> TestResult {
  let client = client("q_pg_fetch_one_empty").await?;
  let Err(QueryError::NotFound { table }) = users_select().fetch_one::<User>(&client).await else {
    return Err("expected QueryError::NotFound".into());
  };
  assert_eq!(table, "users");
  Ok(())
}

#[tokio::test]
async fn fetch_optional_none_then_some() -> TestResult {
  let client = client("q_pg_fetch_optional").await?;
  assert_eq!(users_select().fetch_optional::<User>(&client).await?, None);
  insert_user(&client, "u1", "Ann", "ann@x.io", None).await?;
  let found = users_select().fetch_optional::<User>(&client).await?;
  assert_eq!(found.map(|u| u.id), Some("u1".to_owned()));
  Ok(())
}

#[tokio::test]
async fn count_and_exists_reflect_real_rows() -> TestResult {
  let client = client("q_pg_count_exists").await?;
  for (id, name) in [("u1", "Ann"), ("u2", "Bea"), ("u3", "Cid")] {
    insert_user(&client, id, name, "x@x.io", None).await?;
  }
  assert_eq!(users_select().count(&client).await?, 3);
  assert!(
    users_select()
      .filter(NAME.eq("Ann"))
      .exists(&client)
      .await?
  );
  assert!(
    !users_select()
      .filter(NAME.eq("Zed"))
      .exists(&client)
      .await?
  );
  Ok(())
}

#[tokio::test]
async fn fetch_one_with_two_matches_returns_the_first_by_order() -> TestResult {
  let client = client("q_pg_fetch_one_two").await?;
  insert_user(&client, "u1", "Ann", "a@x.io", None).await?;
  insert_user(&client, "u2", "Ann", "b@x.io", None).await?;
  let user = users_select()
    .filter(NAME.eq("Ann"))
    .order_by(ID.desc())
    .fetch_one::<User>(&client)
    .await?;
  assert_eq!(user.id, "u2");
  Ok(())
}

/// u1 Ann 30 a@x.io · u2 Bea 25 b@x.io · u3 Cid NULL c@y.io · u4 Dee 41 d@y.io
async fn seeded(schema: &str) -> Result<tokio_postgres::Client, Box<dyn std::error::Error>> {
  let client = client(schema).await?;
  insert_user(&client, "u1", "Ann", "a@x.io", Some(30)).await?;
  insert_user(&client, "u2", "Bea", "b@x.io", Some(25)).await?;
  insert_user(&client, "u3", "Cid", "c@y.io", None).await?;
  insert_user(&client, "u4", "Dee", "d@y.io", Some(41)).await?;
  Ok(client)
}

async fn ids(
  client: &tokio_postgres::Client,
  filters: Vec<Expr>,
) -> Result<Vec<String>, QueryError> {
  let mut select = users_select().order_by(ID.asc());
  for f in filters {
    select = select.filter(f);
  }
  let users = select.fetch_all::<User>(client).await?;
  Ok(users.into_iter().map(|u| u.id).collect())
}

fn text(s: &str) -> Value {
  Value::Text(s.into())
}

#[tokio::test]
async fn every_filter_operator_selects_the_expected_rows() -> TestResult {
  let client = seeded("q_pg_filters").await?;
  let cases: Vec<(&str, Expr, &[&str])> = vec![
    ("eq", NAME.eq("Bea"), &["u2"]),
    ("ne", NAME.ne("Bea"), &["u1", "u3", "u4"]),
    (
      "in_list",
      ID.in_list(&[text("u1"), text("u4")]),
      &["u1", "u4"],
    ),
    (
      "not_in",
      ID.not_in(&[text("u1"), text("u4")]),
      &["u2", "u3"],
    ),
    ("is_null", AGE.is_null(), &["u3"]),
    ("is_not_null", AGE.is_not_null(), &["u1", "u2", "u4"]),
    ("like", EMAIL.like("%@y.io"), &["u3", "u4"]),
    ("gt", AGE.gt(30_i64), &["u4"]),
    ("lt", AGE.lt(30_i64), &["u2"]),
    ("gte", AGE.gte(30_i64), &["u1", "u4"]),
    ("lte", AGE.lte(30_i64), &["u1", "u2"]),
    ("between", AGE.between(25_i64, 30_i64), &["u1", "u2"]),
  ];
  for (op, expr, expected) in cases {
    assert_eq!(ids(&client, vec![expr]).await?, expected, "operator {op}");
  }
  Ok(())
}

#[tokio::test]
async fn three_filters_keep_numbering_params_past_the_first_two() -> TestResult {
  let client = seeded("q_pg_three_filters").await?;
  let found = ids(
    &client,
    vec![
      AGE.gte(25_i64),
      EMAIL.like("%x.io"),
      ID.in_list(&[text("u1"), text("u2"), text("u3")]),
    ],
  )
  .await?;
  assert_eq!(found, ["u1", "u2"]);
  Ok(())
}

#[tokio::test]
async fn nested_and_or_keeps_precedence() -> TestResult {
  let client = seeded("q_pg_nested").await?;
  let expr = NAME.eq("Ann").and(AGE.gt(20_i64)).or(NAME.eq("Dee"));
  assert_eq!(ids(&client, vec![expr]).await?, ["u1", "u4"]);
  Ok(())
}

#[tokio::test]
async fn empty_in_list_executes_and_matches_nothing() -> TestResult {
  let client = seeded("q_pg_empty_in").await?;
  let found = ids(&client, vec![ID.in_list(&[])]).await?;
  assert!(found.is_empty(), "got {found:?}");
  Ok(())
}

#[tokio::test]
async fn empty_not_in_matches_every_row() -> TestResult {
  let client = seeded("q_pg_empty_not_in").await?;
  let found = ids(&client, vec![ID.not_in(&[])]).await?;
  assert_eq!(found, ["u1", "u2", "u3", "u4"]);
  Ok(())
}
