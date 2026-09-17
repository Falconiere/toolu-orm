//! Creating the internal `_migrations` table if it is not there yet.

use toolu_orm_connection::{DbConnection, DbConnectionBlocking};
use toolu_orm_core::dialect::Dialect;

use crate::migrate::ddl::migrations_table_ddl;
use crate::migrate::error::{map_db, MigrateError};

/// # Errors
///
/// Returns [`MigrateError::Database`] if DDL execution fails.
pub async fn ensure_migrations_table(
  conn: &impl DbConnection,
  dialect: Dialect,
) -> Result<(), MigrateError> {
  let ddl = migrations_table_ddl(dialect);
  conn.execute_batch(&ddl).await.map_err(|e| map_db(&e))?;
  Ok(())
}

/// Blocking twin of [`ensure_migrations_table`].
///
/// # Errors
///
/// Returns [`MigrateError::Database`] if DDL execution fails.
pub fn ensure_migrations_table_blocking(
  conn: &impl DbConnectionBlocking,
  dialect: Dialect,
) -> Result<(), MigrateError> {
  let ddl = migrations_table_ddl(dialect);
  conn.execute_batch(&ddl).map_err(|e| map_db(&e))?;
  Ok(())
}
