//! In-memory libsql connection with `users`/`posts` DDL, shared by every
//! libsql execution-scenario binary (mutations, reads, relational).

pub(crate) async fn setup_db() -> Result<libsql::Connection, Box<dyn std::error::Error>> {
  let db = libsql::Builder::new_local(":memory:").build().await?;
  let conn = db.connect()?;
  conn
    .execute(
      "CREATE TABLE users (id TEXT PRIMARY KEY, name TEXT NOT NULL, email TEXT NOT NULL, age INTEGER)",
      (),
    )
    .await?;
  conn
    .execute(
      "CREATE TABLE posts (id TEXT PRIMARY KEY, author_id TEXT REFERENCES users(id), title TEXT NOT NULL)",
      (),
    )
    .await?;
  Ok(conn)
}
