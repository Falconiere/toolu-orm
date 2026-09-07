//! PgCountScalar for COUNT/EXISTS scalar queries on Postgres.

use crate::error::DbCoreError;

/// Scalar row for `COUNT(*)` / `EXISTS` on Postgres (`orm-query` executor path).
#[cfg(feature = "postgres")]
pub struct PgCountScalar {
  pub value: i64,
}

#[cfg(feature = "postgres")]
fn from_pg(row: &tokio_postgres::Row) -> Result<PgCountScalar, DbCoreError> {
  let value: i64 = row
    .try_get::<_, i64>(0)
    .or_else(|_| row.try_get::<_, i32>(0).map(i64::from))
    .or_else(|_| row.try_get::<_, bool>(0).map(i64::from))
    .map_err(|e| DbCoreError::RowMapping(format!("scalar: {e}")))?;
  Ok(PgCountScalar { value })
}

#[cfg(feature = "libsql")]
fn libsql_unsupported(_: &libsql::Row) -> Result<PgCountScalar, DbCoreError> {
  Err(DbCoreError::RowMapping(
    "PgCountScalar is only used on the Postgres executor path".into(),
  ))
}

#[cfg(feature = "rusqlite")]
fn rusqlite_unsupported(_: &rusqlite::Row<'_>) -> Result<PgCountScalar, DbCoreError> {
  Err(DbCoreError::RowMapping(
    "PgCountScalar is only used on the Postgres executor path".into(),
  ))
}

crate::impl_from_row_for!(single
  cfg(all(feature = "postgres", not(feature = "libsql"), not(feature = "rusqlite"))),
  PgCountScalar, &["scalar"], from_row, tokio_postgres::Row, from_pg);

crate::impl_from_row_for!(dual
  cfg(all(feature = "postgres", feature = "rusqlite", not(feature = "libsql"))),
  PgCountScalar, &["scalar"],
  [from_pg_row tokio_postgres::Row => from_pg],
  [from_rusqlite_row rusqlite::Row<'_> => rusqlite_unsupported]);

crate::impl_from_row_for!(dual
  cfg(all(feature = "postgres", feature = "libsql", not(feature = "rusqlite"))),
  PgCountScalar, &["scalar"],
  [from_pg_row tokio_postgres::Row => from_pg],
  [from_libsql_row libsql::Row => libsql_unsupported]);

crate::impl_from_row_for!(triple
  cfg(all(feature = "postgres", feature = "libsql", feature = "rusqlite")),
  PgCountScalar, &["scalar"],
  [from_pg_row tokio_postgres::Row => from_pg],
  [from_libsql_row libsql::Row => libsql_unsupported],
  [from_rusqlite_row rusqlite::Row<'_> => rusqlite_unsupported]);
