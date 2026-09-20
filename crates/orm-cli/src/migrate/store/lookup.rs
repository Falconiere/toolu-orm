//! Reading back what `_migrations` already holds.

use toolu_orm_connection::{DbConnection, DbConnectionBlocking};

use super::applied::AppliedMigration;
use crate::migrate::error::{map_db, MigrateError};
use crate::migrate::sql::SELECT_APPLIED_MIGRATIONS;

/// The names of every applied migration, in the order they were applied.
///
/// Reads the same rows as `get_applied_history` and drops their hashes: a
/// caller that only reports names ([`crate::status`]) needs no second
/// statement, and the runners that do need the hashes take the records whole.
///
/// # Errors
///
/// Returns [`MigrateError::Database`] if the query fails.
pub async fn get_applied_migrations(conn: &impl DbConnection) -> Result<Vec<String>, MigrateError> {
  Ok(names(get_applied_history(conn).await?))
}

/// Blocking twin of [`get_applied_migrations`].
///
/// # Errors
///
/// Returns [`MigrateError::Database`] if the query fails.
pub fn get_applied_migrations_blocking(
  conn: &impl DbConnectionBlocking,
) -> Result<Vec<String>, MigrateError> {
  Ok(names(get_applied_history_blocking(conn)?))
}

/// Every applied migration with the hash recorded when it ran.
///
/// The runners need the hashes, not just the names: an entry they are about to
/// skip is first checked against the history its source declares
/// ([`crate::migrate::history`]). One statement serves both, so a run still
/// reads `_migrations` exactly once.
pub(crate) async fn get_applied_history(
  conn: &impl DbConnection,
) -> Result<Vec<AppliedMigration>, MigrateError> {
  conn
    .query_map::<AppliedMigration>(SELECT_APPLIED_MIGRATIONS, vec![])
    .await
    .map_err(|e| map_db(&e))
}

/// Blocking twin of `get_applied_history`.
pub(crate) fn get_applied_history_blocking(
  conn: &impl DbConnectionBlocking,
) -> Result<Vec<AppliedMigration>, MigrateError> {
  conn
    .query_map::<AppliedMigration>(SELECT_APPLIED_MIGRATIONS, vec![])
    .map_err(|e| map_db(&e))
}

fn names(records: Vec<AppliedMigration>) -> Vec<String> {
  records.into_iter().map(|record| record.name).collect()
}
