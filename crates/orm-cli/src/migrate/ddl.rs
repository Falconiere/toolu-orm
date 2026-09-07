//! Dialect-aware DDL for the internal `_migrations` table.

use toolu_orm_core::dialect::Dialect;

/// DDL for the internal `_migrations` table (dialect-specific).
#[must_use]
pub fn migrations_table_ddl(dialect: Dialect) -> String {
  match dialect {
    Dialect::Sqlite => "CREATE TABLE IF NOT EXISTS _migrations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                hash TEXT NOT NULL DEFAULT '',
                applied_at INTEGER NOT NULL DEFAULT (unixepoch())
            );"
      .to_owned(),
    Dialect::Postgres => "CREATE TABLE IF NOT EXISTS _migrations (
                id SERIAL PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                hash TEXT NOT NULL DEFAULT '',
                applied_at BIGINT NOT NULL DEFAULT extract(epoch from now())::bigint
            );"
      .to_owned(),
  }
}
