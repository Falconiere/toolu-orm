//! Migration record storage, lookup, and table initialization.

use toolu_orm_connection::{DbConnection, DbConnectionBlocking};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::value::Value;

use super::ddl::migrations_table_ddl;
use super::error::{map_db, MigrateError};
use super::sql::{insert_migration_sql, SELECT_APPLIED_MIGRATIONS};

struct AppliedMigration {
  name: String,
}

#[cfg(orm_core_has_postgres)]
fn map_pg(row: &toolu_orm_core::tokio_postgres::Row) -> Result<AppliedMigration, DbCoreError> {
  let name: String = row
    .try_get(0)
    .map_err(|e: toolu_orm_core::tokio_postgres::Error| DbCoreError::RowMapping(e.to_string()))?;
  Ok(AppliedMigration { name })
}

#[cfg(orm_core_has_libsql)]
fn map_libsql(row: &toolu_orm_core::libsql::Row) -> Result<AppliedMigration, DbCoreError> {
  let name: String = row
    .get::<String>(0)
    .map_err(|e| DbCoreError::RowMapping(e.to_string()))?;
  Ok(AppliedMigration { name })
}

#[cfg(orm_core_has_rusqlite)]
fn map_rusqlite(row: &toolu_orm_core::rusqlite::Row<'_>) -> Result<AppliedMigration, DbCoreError> {
  let name: String = row
    .get(0)
    .map_err(|e| DbCoreError::RowMapping(e.to_string()))?;
  Ok(AppliedMigration { name })
}

// The `FromRow` trait shape depends on `toolu-orm-core`'s unified features,
// NOT on `orm-cli`'s own features. We use `orm_core_has_*` cfgs set by our
// build.rs (reading DEP_TOOLU_ORM_CORE_* metadata from orm-core's build script)
// to match the exact trait shape orm-core compiled.
toolu_orm_core::impl_from_row_for!(single
  cfg(all(orm_core_has_libsql, not(orm_core_has_postgres), not(orm_core_has_rusqlite))),
  AppliedMigration, &["name"], from_row, toolu_orm_core::libsql::Row, map_libsql);

toolu_orm_core::impl_from_row_for!(single
  cfg(all(orm_core_has_postgres, not(orm_core_has_libsql), not(orm_core_has_rusqlite))),
  AppliedMigration, &["name"], from_row, toolu_orm_core::tokio_postgres::Row, map_pg);

toolu_orm_core::impl_from_row_for!(single
  cfg(all(orm_core_has_rusqlite, not(orm_core_has_libsql), not(orm_core_has_postgres))),
  AppliedMigration, &["name"], from_row, toolu_orm_core::rusqlite::Row<'_>, map_rusqlite);

toolu_orm_core::impl_from_row_for!(dual
  cfg(all(orm_core_has_postgres, orm_core_has_libsql, not(orm_core_has_rusqlite))),
  AppliedMigration, &["name"],
  [from_pg_row toolu_orm_core::tokio_postgres::Row => map_pg],
  [from_libsql_row toolu_orm_core::libsql::Row => map_libsql]);

toolu_orm_core::impl_from_row_for!(dual
  cfg(all(orm_core_has_postgres, orm_core_has_rusqlite, not(orm_core_has_libsql))),
  AppliedMigration, &["name"],
  [from_pg_row toolu_orm_core::tokio_postgres::Row => map_pg],
  [from_rusqlite_row toolu_orm_core::rusqlite::Row<'_> => map_rusqlite]);

toolu_orm_core::impl_from_row_for!(dual
  cfg(all(orm_core_has_libsql, orm_core_has_rusqlite, not(orm_core_has_postgres))),
  AppliedMigration, &["name"],
  [from_libsql_row toolu_orm_core::libsql::Row => map_libsql],
  [from_rusqlite_row toolu_orm_core::rusqlite::Row<'_> => map_rusqlite]);

toolu_orm_core::impl_from_row_for!(triple
  cfg(all(orm_core_has_postgres, orm_core_has_libsql, orm_core_has_rusqlite)),
  AppliedMigration, &["name"],
  [from_pg_row toolu_orm_core::tokio_postgres::Row => map_pg],
  [from_libsql_row toolu_orm_core::libsql::Row => map_libsql],
  [from_rusqlite_row toolu_orm_core::rusqlite::Row<'_> => map_rusqlite]);

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

/// # Errors
///
/// Returns [`MigrateError::Database`] if the query fails.
pub async fn get_applied_migrations(conn: &impl DbConnection) -> Result<Vec<String>, MigrateError> {
  let rows = conn
    .query_map::<AppliedMigration>(SELECT_APPLIED_MIGRATIONS, vec![])
    .await
    .map_err(|e| map_db(&e))?;
  Ok(rows.into_iter().map(|r| r.name).collect())
}

/// Blocking twin of [`get_applied_migrations`].
///
/// # Errors
///
/// Returns [`MigrateError::Database`] if the query fails.
pub fn get_applied_migrations_blocking(
  conn: &impl DbConnectionBlocking,
) -> Result<Vec<String>, MigrateError> {
  let rows = conn
    .query_map::<AppliedMigration>(SELECT_APPLIED_MIGRATIONS, vec![])
    .map_err(|e| map_db(&e))?;
  Ok(rows.into_iter().map(|r| r.name).collect())
}

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
