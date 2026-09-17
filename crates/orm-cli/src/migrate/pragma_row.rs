//! Decoding a one-integer-column row, for the SQLite pragmas the runner reads.
//!
//! `PRAGMA foreign_keys` and `SELECT count(*) FROM pragma_foreign_key_check`
//! both answer with a single integer. The `FromRow` shape depends on
//! `toolu-orm-core`'s *unified* features rather than this crate's, so the
//! `orm_core_has_*` cfgs from `build.rs` pick the matching arm — the same
//! reasoning as [`super::store`].

use toolu_orm_core::error::DbCoreError;

/// One integer column, read positionally.
pub(super) struct PragmaInt {
  /// The pragma's value, or the violation count.
  pub value: i64,
}

#[cfg(orm_core_has_postgres)]
fn map_pg(row: &toolu_orm_core::tokio_postgres::Row) -> Result<PragmaInt, DbCoreError> {
  let value: i64 = row
    .try_get(0)
    .map_err(|e: toolu_orm_core::tokio_postgres::Error| DbCoreError::RowMapping(e.to_string()))?;
  Ok(PragmaInt { value })
}

#[cfg(orm_core_has_libsql)]
fn map_libsql(row: &toolu_orm_core::libsql::Row) -> Result<PragmaInt, DbCoreError> {
  let value: i64 = row
    .get::<i64>(0)
    .map_err(|e| DbCoreError::RowMapping(e.to_string()))?;
  Ok(PragmaInt { value })
}

#[cfg(orm_core_has_rusqlite)]
fn map_rusqlite(row: &toolu_orm_core::rusqlite::Row<'_>) -> Result<PragmaInt, DbCoreError> {
  let value: i64 = row
    .get(0)
    .map_err(|e| DbCoreError::RowMapping(e.to_string()))?;
  Ok(PragmaInt { value })
}

toolu_orm_core::impl_from_row_for!(single
  cfg(all(orm_core_has_libsql, not(orm_core_has_postgres), not(orm_core_has_rusqlite))),
  PragmaInt, &[], from_row, toolu_orm_core::libsql::Row, map_libsql);

toolu_orm_core::impl_from_row_for!(single
  cfg(all(orm_core_has_postgres, not(orm_core_has_libsql), not(orm_core_has_rusqlite))),
  PragmaInt, &[], from_row, toolu_orm_core::tokio_postgres::Row, map_pg);

toolu_orm_core::impl_from_row_for!(single
  cfg(all(orm_core_has_rusqlite, not(orm_core_has_libsql), not(orm_core_has_postgres))),
  PragmaInt, &[], from_row, toolu_orm_core::rusqlite::Row<'_>, map_rusqlite);

toolu_orm_core::impl_from_row_for!(dual
  cfg(all(orm_core_has_postgres, orm_core_has_libsql, not(orm_core_has_rusqlite))),
  PragmaInt, &[],
  [from_pg_row toolu_orm_core::tokio_postgres::Row => map_pg],
  [from_libsql_row toolu_orm_core::libsql::Row => map_libsql]);

toolu_orm_core::impl_from_row_for!(dual
  cfg(all(orm_core_has_postgres, orm_core_has_rusqlite, not(orm_core_has_libsql))),
  PragmaInt, &[],
  [from_pg_row toolu_orm_core::tokio_postgres::Row => map_pg],
  [from_rusqlite_row toolu_orm_core::rusqlite::Row<'_> => map_rusqlite]);

toolu_orm_core::impl_from_row_for!(dual
  cfg(all(orm_core_has_libsql, orm_core_has_rusqlite, not(orm_core_has_postgres))),
  PragmaInt, &[],
  [from_libsql_row toolu_orm_core::libsql::Row => map_libsql],
  [from_rusqlite_row toolu_orm_core::rusqlite::Row<'_> => map_rusqlite]);

toolu_orm_core::impl_from_row_for!(triple
  cfg(all(orm_core_has_postgres, orm_core_has_libsql, orm_core_has_rusqlite)),
  PragmaInt, &[],
  [from_pg_row toolu_orm_core::tokio_postgres::Row => map_pg],
  [from_libsql_row toolu_orm_core::libsql::Row => map_libsql],
  [from_rusqlite_row toolu_orm_core::rusqlite::Row<'_> => map_rusqlite]);
