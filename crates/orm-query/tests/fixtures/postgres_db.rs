//! Live-Postgres fixtures for the orm-query postgres lane.
//!
//! orm-query has no dependency on orm-connection, so this reads the same
//! `TEST_DB_*` variables as `PgConfig::for_test` (defaults `localhost`, `5433`,
//! `toolu`/`toolu`, db `toolu`) and opens a raw `tokio_postgres::Client`. Every
//! test owns a schema. Wired into each `postgres_*_test.rs` binary as a
//! `pub mod` via `#[path]`.

use toolu_orm_core::column::{Integer, Text};
// The FromRow derive names `libsql::Row`; this lane gets it via orm-core.
use toolu_orm_core::libsql;
use toolu_orm_core::query_column::Column;
use toolu_orm_core::row::FromRow as _;
use toolu_orm_macros::FromRow;
use toolu_orm_query::executor::Executor;
use toolu_orm_query::insert::InsertBuilder;
use toolu_orm_query::select::SelectBuilder;
use toolu_orm_query::QueryError;

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

pub const ID: Column<Text> = Column::new("users", "id");
pub const NAME: Column<Text> = Column::new("users", "name");
pub const EMAIL: Column<Text> = Column::new("users", "email");
pub const AGE: Column<Integer> = Column::new("users", "age");

const DDL: &str = "CREATE TABLE users (id TEXT PRIMARY KEY, name TEXT NOT NULL, \
   email TEXT NOT NULL, age BIGINT); \
   CREATE TABLE posts (id TEXT PRIMARY KEY, author_id TEXT REFERENCES users(id), \
   title TEXT NOT NULL)";

/// Decoded positionally by the derive, so selects must list the columns in
/// this order (`User::REQUIRED_COLUMNS`).
#[derive(FromRow, Debug, Clone, PartialEq)]
pub struct User {
  pub id: String,
  pub name: String,
  pub email: String,
  pub age: Option<i64>,
}

fn env_or(key: &str, default: &str) -> String {
  std::env::var(key).unwrap_or_else(|_| default.to_owned())
}

/// Connects, drops and recreates `schema`, sets `search_path`, creates the
/// fixture tables. Fails hard when the server is unreachable.
///
/// # Errors
///
/// Connection or DDL failure.
pub async fn client(schema: &str) -> Result<tokio_postgres::Client, Box<dyn std::error::Error>> {
  let conn_str = format!(
    "host={} port={} user={} password={} dbname={}",
    env_or("TEST_DB_HOST", "localhost"),
    env_or("TEST_DB_PORT", "5433"),
    env_or("TEST_DB_USER", "toolu"),
    env_or("TEST_DB_PASSWORD", "toolu"),
    env_or("TEST_DB_NAME", "toolu"),
  );
  let (client, connection) = tokio_postgres::connect(&conn_str, tokio_postgres::NoTls).await?;
  tokio::spawn(async move {
    if let Err(e) = connection.await {
      eprintln!("postgres connection task ended with error: {e}");
    }
  });
  client
    .batch_execute(&format!(
      "DROP SCHEMA IF EXISTS {schema} CASCADE; CREATE SCHEMA {schema}; \
       SET search_path TO {schema}; {DDL}"
    ))
    .await?;
  Ok(client)
}

/// `SELECT` over `users` in `User` field order.
pub fn users_select() -> SelectBuilder {
  SelectBuilder::new("users").columns_raw(User::REQUIRED_COLUMNS)
}

/// # Errors
///
/// Driver error.
pub async fn insert_user(
  exec: &(impl Executor + Send + Sync),
  id: &str,
  name: &str,
  email: &str,
  age: Option<i64>,
) -> Result<u64, QueryError> {
  let builder = InsertBuilder::new("users")
    .set(&ID, id)
    .set(&NAME, name)
    .set(&EMAIL, email);
  let builder = match age {
    Some(age) => builder.set(&AGE, age),
    None => builder.set_null(&AGE),
  };
  builder.execute(exec).await
}

/// All users ordered by id.
///
/// # Errors
///
/// Driver or row-mapping error.
pub async fn all_users(exec: &(impl Executor + Send + Sync)) -> Result<Vec<User>, QueryError> {
  users_select()
    .order_by(ID.asc())
    .fetch_all::<User>(exec)
    .await
}
