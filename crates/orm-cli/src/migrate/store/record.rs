//! Recording one migration as applied.

use toolu_orm_connection::{DbConnection, DbConnectionBlocking};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::value::Value;

use crate::migrate::error::{map_db, MigrateError};
use crate::migrate::sql::insert_migration_sql;

/// # Errors
///
/// Returns [`MigrateError::Database`] if the insert fails.
pub async fn record_migration(
  conn: &impl DbConnection,
  name: &str,
  hash: &str,
  dialect: Dialect,
) -> Result<(), MigrateError> {
  conn
    .execute_sql(
      insert_migration_sql(dialect),
      vec![Value::Text(name.to_owned()), Value::Text(hash.to_owned())],
    )
    .await
    .map_err(|e| map_db(&e))?;
  Ok(())
}

/// Blocking twin of [`record_migration`].
///
/// # Errors
///
/// Returns [`MigrateError::Database`] if the insert fails.
pub fn record_migration_blocking(
  conn: &impl DbConnectionBlocking,
  name: &str,
  hash: &str,
  dialect: Dialect,
) -> Result<(), MigrateError> {
  conn
    .execute_sql(
      insert_migration_sql(dialect),
      vec![Value::Text(name.to_owned()), Value::Text(hash.to_owned())],
    )
    .map_err(|e| map_db(&e))?;
  Ok(())
}
