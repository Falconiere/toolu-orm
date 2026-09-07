//! Generate → migrate → evolve → generate → migrate against in-memory libsql,
//! asserting the applied schema through `PRAGMA` and `sqlite_master` rather
//! than through the generated SQL text.

#[path = "fixtures/loop_registry.rs"]
pub mod loop_registry;

use toolu_orm_cli::generate::run_generate;
use toolu_orm_cli::migrate::run_migrate;
use toolu_orm_cli::status::get_status;
use toolu_orm_connection::{Database, DbConnection, LibsqlConnection};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::value::Value;

use loop_registry::{migrations_dir, registry_v1, registry_v2};

type TestResult = Result<(), Box<dyn std::error::Error>>;

async fn connect() -> Result<LibsqlConnection, Box<dyn std::error::Error>> {
  Ok(Database::init_local(":memory:").await?.connect()?)
}

async fn column_names(
  conn: &LibsqlConnection,
  table: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
  let mut rows = conn
    .inner_conn()
    .query(&format!("PRAGMA table_info({table})"), ())
    .await?;
  let mut names = Vec::new();
  while let Some(row) = rows.next().await? {
    names.push(row.get::<String>(1)?);
  }
  Ok(names)
}

async fn scalar(conn: &LibsqlConnection, sql: &str) -> Result<i64, Box<dyn std::error::Error>> {
  let mut rows = conn.inner_conn().query(sql, ()).await?;
  let row = rows.next().await?.ok_or("scalar query returned no row")?;
  Ok(row.get::<i64>(0)?)
}

/// v1 → v2 through real generate/migrate calls; returns the connection.
async fn evolve_to_v2(dir: &str) -> Result<LibsqlConnection, Box<dyn std::error::Error>> {
  let conn = connect().await?;
  run_generate(&registry_v1(), dir, "init", Dialect::Sqlite)?;
  run_migrate(&conn, dir, Dialect::Sqlite).await?;
  run_generate(&registry_v2(), dir, "evolve", Dialect::Sqlite)?;
  run_migrate(&conn, dir, Dialect::Sqlite).await?;
  Ok(conn)
}

#[tokio::test]
async fn loop_v1_then_v2_applies_every_change() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = connect().await?;

  let first = run_generate(&registry_v1(), &dir, "init", Dialect::Sqlite)?;
  assert_eq!(first.as_deref(), Some("0001_init.sql"));
  assert_eq!(run_migrate(&conn, &dir, Dialect::Sqlite).await?, 1);
  assert_eq!(
    column_names(&conn, "users").await?,
    ["id", "name", "email", "age"]
  );

  let second = run_generate(&registry_v2(), &dir, "evolve", Dialect::Sqlite)?;
  assert_eq!(second.as_deref(), Some("0002_evolve.sql"));
  assert_eq!(run_migrate(&conn, &dir, Dialect::Sqlite).await?, 1);

  let cols = column_names(&conn, "users").await?;
  assert!(cols.iter().any(|c| c == "bio"), "bio missing: {cols:?}");
  let post_cols = column_names(&conn, "posts").await?;
  assert_eq!(post_cols, ["id", "author_id", "title", "status"]);
  assert_eq!(
    scalar(
      &conn,
      "SELECT count(*) FROM sqlite_master WHERE type = 'index' AND name = 'idx_users_email'"
    )
    .await?,
    1
  );
  assert_eq!(
    scalar(
      &conn,
      "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'posts'"
    )
    .await?,
    1
  );
  assert_eq!(
    scalar(
      &conn,
      "SELECT count(*) FROM pragma_foreign_key_list('posts') WHERE \"table\" = 'users'"
    )
    .await?,
    1
  );

  let status = get_status(&conn, &dir, Dialect::Sqlite).await?;
  assert_eq!(status.applied, ["0001_init.sql", "0002_evolve.sql"]);
  assert!(status.pending.is_empty(), "pending: {:?}", status.pending);
  assert_eq!(
    run_generate(&registry_v2(), &dir, "noop", Dialect::Sqlite)?,
    None
  );
  Ok(())
}

#[tokio::test]
async fn evolved_schema_enforces_enum_check_and_unique_email() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = evolve_to_v2(&dir).await?;
  let user = |id: &str, email: &str| {
    vec![
      Value::Text(id.into()),
      Value::Text("Ann".into()),
      Value::Text(email.into()),
    ]
  };
  let post = |id: &str, status: &str| {
    vec![
      Value::Text(id.into()),
      Value::Text("u1".into()),
      Value::Text("Hello".into()),
      Value::Text(status.into()),
    ]
  };
  let insert_user = "INSERT INTO users (id, name, email) VALUES (?1, ?2, ?3)";
  let insert_post = "INSERT INTO posts (id, author_id, title, status) VALUES (?1, ?2, ?3, ?4)";

  assert_eq!(
    conn
      .execute_sql(insert_user, user("u1", "ann@x.io"))
      .await?,
    1
  );
  assert_eq!(
    conn.execute_sql(insert_post, post("p1", "active")).await?,
    1
  );
  let bad_status = conn.execute_sql(insert_post, post("p2", "bogus")).await;
  assert!(bad_status.is_err(), "CHECK on posts.status was not applied");
  let dup_email = conn.execute_sql(insert_user, user("u2", "ann@x.io")).await;
  assert!(dup_email.is_err(), "unique index on email was not applied");
  assert_eq!(scalar(&conn, "SELECT count(*) FROM users").await?, 1);
  assert_eq!(scalar(&conn, "SELECT count(*) FROM posts").await?, 1);
  Ok(())
}

#[tokio::test]
async fn evolved_schema_cascades_post_deletes() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = evolve_to_v2(&dir).await?;
  conn.execute_batch("PRAGMA foreign_keys = ON").await?;
  conn
    .execute_batch(
      "INSERT INTO users (id, name, email) VALUES ('u1', 'Ann', 'ann@x.io'); \
       INSERT INTO posts (id, author_id, title) VALUES ('p1', 'u1', 'Hello')",
    )
    .await?;
  assert_eq!(
    conn
      .execute_sql(
        "DELETE FROM users WHERE id = ?1",
        vec![Value::Text("u1".into())]
      )
      .await?,
    1
  );
  assert_eq!(scalar(&conn, "SELECT count(*) FROM posts").await?, 0);
  Ok(())
}
