//! Blocking twin of [`super::pragma_guard`], over [`DbConnectionBlocking`].

use toolu_orm_connection::DbConnectionBlocking;
use toolu_orm_core::dialect::Dialect;

use super::error::{map_db, MigrateError};
use super::pragma_guard::{
  enforced_before, flag, manages_pragmas, restore_failure, restore_sql, PragmaGuard,
};
use super::pragma_row::PragmaInt;
use super::sql::{
  COUNT_FOREIGN_KEY_VIOLATIONS, DISABLE_FOREIGN_KEYS, READ_FOREIGN_KEYS, READ_LEGACY_ALTER_TABLE,
};

fn read_pragma(
  conn: &impl DbConnectionBlocking,
  sql: &str,
) -> Result<Vec<PragmaInt>, MigrateError> {
  conn
    .query_map::<PragmaInt>(sql, vec![])
    .map_err(|e| map_db(&e))
}

/// Blocking twin of [`super::pragma_guard::arm_pragmas`].
pub(super) fn arm_pragmas(
  conn: &impl DbConnectionBlocking,
  sql: &str,
  dialect: Dialect,
) -> Result<PragmaGuard, MigrateError> {
  if !manages_pragmas(sql, dialect) {
    return Ok(PragmaGuard::Untouched);
  }
  let foreign_keys = flag(&read_pragma(conn, READ_FOREIGN_KEYS)?);
  let legacy_alter_table = flag(&read_pragma(conn, READ_LEGACY_ALTER_TABLE)?);
  if foreign_keys {
    conn
      .execute_batch(DISABLE_FOREIGN_KEYS)
      .map_err(|e| map_db(&e))?;
  }
  Ok(PragmaGuard::Armed {
    foreign_keys,
    legacy_alter_table,
  })
}

/// Blocking twin of [`super::pragma_guard::check_foreign_keys`].
pub(super) fn check_foreign_keys(
  conn: &impl DbConnectionBlocking,
  guard: PragmaGuard,
  file: &str,
) -> Result<(), MigrateError> {
  if !enforced_before(guard) {
    return Ok(());
  }
  let rows = read_pragma(conn, COUNT_FOREIGN_KEY_VIOLATIONS)?;
  let count = rows.first().map_or(0, |row| row.value);
  if count == 0 {
    return Ok(());
  }
  Err(MigrateError::ForeignKeyViolation {
    file: file.to_owned(),
    count,
  })
}

/// Blocking twin of [`super::pragma_guard::restore_pragmas`].
pub(super) fn restore_pragmas(
  conn: &impl DbConnectionBlocking,
  guard: PragmaGuard,
  outcome: Result<(), MigrateError>,
) -> Result<(), MigrateError> {
  let PragmaGuard::Armed {
    foreign_keys,
    legacy_alter_table,
  } = guard
  else {
    return outcome;
  };
  match conn.execute_batch(&restore_sql(foreign_keys, legacy_alter_table)) {
    Ok(()) => outcome,
    Err(restore_err) => Err(restore_failure(outcome, &restore_err.to_string())),
  }
}
