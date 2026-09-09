//! SQL strings shared by the async and blocking migrate I/O paths.

use toolu_orm_core::dialect::Dialect;

/// INSERT used by [`super::store::record_migration`] and its blocking twin.
#[must_use]
pub(super) fn insert_migration_sql(dialect: Dialect) -> &'static str {
  match dialect {
    Dialect::Sqlite => "INSERT INTO _migrations (name, hash) VALUES (?1, ?2)",
    Dialect::Postgres => "INSERT INTO _migrations (name, hash) VALUES ($1, $2)",
  }
}

/// SELECT used by [`super::store::get_applied_migrations`] and its blocking twin.
pub(super) const SELECT_APPLIED_MIGRATIONS: &str = "SELECT name FROM _migrations ORDER BY id";

pub(super) const BEGIN: &str = "BEGIN";
pub(super) const COMMIT: &str = "COMMIT";
pub(super) const ROLLBACK: &str = "ROLLBACK";
