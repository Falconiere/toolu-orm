//! One recorded `_migrations` row, decoded from whichever driver shape
//! `toolu-orm-core` compiled.

use toolu_orm_core::error::DbCoreError;

/// A migration the database has already applied.
///
/// `hash` is what was recorded when the migration ran, and what
/// [`crate::migrate::history`] compares against the hash the journal or the
/// embedded list declares today. It is empty for a row the journal-free runner
/// wrote, for a row recorded before hashes existed, and for a `NULL` — all of
/// which are the legacy case: unverifiable rather than verified.
pub(crate) struct AppliedMigration {
  pub name: String,
  pub hash: String,
}

// `hash` is decoded as `Option<String>` on every driver: `NULL` and `''` both
// mean "no hash was recorded", so a hand-created `_migrations` reads as the
// legacy case instead of a row-mapping failure.
#[cfg(orm_core_has_postgres)]
fn map_pg(row: &toolu_orm_core::tokio_postgres::Row) -> Result<AppliedMigration, DbCoreError> {
  let name: String = row
    .try_get(0)
    .map_err(|e: toolu_orm_core::tokio_postgres::Error| DbCoreError::RowMapping(e.to_string()))?;
  let hash: Option<String> = row
    .try_get(1)
    .map_err(|e: toolu_orm_core::tokio_postgres::Error| DbCoreError::RowMapping(e.to_string()))?;
  Ok(AppliedMigration {
    name,
    hash: hash.unwrap_or_default(),
  })
}

#[cfg(orm_core_has_libsql)]
fn map_libsql(row: &toolu_orm_core::libsql::Row) -> Result<AppliedMigration, DbCoreError> {
  let name: String = row
    .get::<String>(0)
    .map_err(|e| DbCoreError::RowMapping(e.to_string()))?;
  let hash: Option<String> = row
    .get::<Option<String>>(1)
    .map_err(|e| DbCoreError::RowMapping(e.to_string()))?;
  Ok(AppliedMigration {
    name,
    hash: hash.unwrap_or_default(),
  })
}

#[cfg(orm_core_has_rusqlite)]
fn map_rusqlite(row: &toolu_orm_core::rusqlite::Row<'_>) -> Result<AppliedMigration, DbCoreError> {
  let name: String = row
    .get(0)
    .map_err(|e| DbCoreError::RowMapping(e.to_string()))?;
  let hash: Option<String> = row
    .get(1)
    .map_err(|e| DbCoreError::RowMapping(e.to_string()))?;
  Ok(AppliedMigration {
    name,
    hash: hash.unwrap_or_default(),
  })
}

// The `FromRow` trait shape depends on `toolu-orm-core`'s unified features,
// NOT on `orm-cli`'s own features. We use `orm_core_has_*` cfgs set by our
// build.rs (reading DEP_TOOLU_ORM_CORE_* metadata from orm-core's build script)
// to match the exact trait shape orm-core compiled.
toolu_orm_core::impl_from_row_for!(single
  cfg(all(orm_core_has_libsql, not(orm_core_has_postgres), not(orm_core_has_rusqlite))),
  AppliedMigration, &["name", "hash"], from_row, toolu_orm_core::libsql::Row, map_libsql);

toolu_orm_core::impl_from_row_for!(single
  cfg(all(orm_core_has_postgres, not(orm_core_has_libsql), not(orm_core_has_rusqlite))),
  AppliedMigration, &["name", "hash"], from_row, toolu_orm_core::tokio_postgres::Row, map_pg);

toolu_orm_core::impl_from_row_for!(single
  cfg(all(orm_core_has_rusqlite, not(orm_core_has_libsql), not(orm_core_has_postgres))),
  AppliedMigration, &["name", "hash"], from_row, toolu_orm_core::rusqlite::Row<'_>, map_rusqlite);

toolu_orm_core::impl_from_row_for!(dual
  cfg(all(orm_core_has_postgres, orm_core_has_libsql, not(orm_core_has_rusqlite))),
  AppliedMigration, &["name", "hash"],
  [from_pg_row toolu_orm_core::tokio_postgres::Row => map_pg],
  [from_libsql_row toolu_orm_core::libsql::Row => map_libsql]);

toolu_orm_core::impl_from_row_for!(dual
  cfg(all(orm_core_has_postgres, orm_core_has_rusqlite, not(orm_core_has_libsql))),
  AppliedMigration, &["name", "hash"],
  [from_pg_row toolu_orm_core::tokio_postgres::Row => map_pg],
  [from_rusqlite_row toolu_orm_core::rusqlite::Row<'_> => map_rusqlite]);

toolu_orm_core::impl_from_row_for!(dual
  cfg(all(orm_core_has_libsql, orm_core_has_rusqlite, not(orm_core_has_postgres))),
  AppliedMigration, &["name", "hash"],
  [from_libsql_row toolu_orm_core::libsql::Row => map_libsql],
  [from_rusqlite_row toolu_orm_core::rusqlite::Row<'_> => map_rusqlite]);

toolu_orm_core::impl_from_row_for!(triple
  cfg(all(orm_core_has_postgres, orm_core_has_libsql, orm_core_has_rusqlite)),
  AppliedMigration, &["name", "hash"],
  [from_pg_row toolu_orm_core::tokio_postgres::Row => map_pg],
  [from_libsql_row toolu_orm_core::libsql::Row => map_libsql],
  [from_rusqlite_row toolu_orm_core::rusqlite::Row<'_> => map_rusqlite]);
