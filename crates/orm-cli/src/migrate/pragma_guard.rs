//! The connection-level pragmas a SQLite table rebuild borrows, and gives back.
//!
//! `PRAGMA foreign_keys` is a no-op inside a transaction, so a rebuild that
//! opens with it would still run with foreign keys enforced and `DROP TABLE`
//! would fire every child's `ON DELETE` action. The runner therefore applies it
//! around the transaction instead. The rebuild also flips
//! `PRAGMA legacy_alter_table` across its final rename, which does work inside
//! the transaction; the guard saves that one too so a failed rename cannot
//! leave the connection with legacy rename semantics.

use toolu_orm_connection::DbConnection;
use toolu_orm_core::dialect::Dialect;

use super::error::{map_db, MigrateError};
use super::pragma_row::PragmaInt;
use super::sql::{
  COUNT_FOREIGN_KEY_VIOLATIONS, DISABLE_FOREIGN_KEYS, READ_FOREIGN_KEYS, READ_LEGACY_ALTER_TABLE,
};

/// What the runner has to put back once the transaction has ended.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum PragmaGuard {
  /// Not a SQLite migration that manages pragmas: nothing read, nothing to
  /// restore.
  Untouched,
  /// The values the connection had before the migration started.
  Armed {
    /// Foreign keys were enforced, and the runner switched them off.
    foreign_keys: bool,
    /// The caller's own `legacy_alter_table` setting.
    legacy_alter_table: bool,
  },
}

/// True when `line` is a `PRAGMA foreign_keys …` statement.
fn is_foreign_keys_statement(line: &str) -> bool {
  let lowered = line.trim().to_lowercase();
  let Some(rest) = lowered.strip_prefix("pragma") else {
    return false;
  };
  rest.trim_start().starts_with("foreign_keys")
}

/// True for a chunk whose only statement is such a pragma. Those chunks are
/// skipped: the runner applies them around the transaction instead, and inside
/// one they would do nothing at all.
pub(super) fn is_foreign_keys_pragma(chunk: &str) -> bool {
  let mut statements = chunk
    .lines()
    .map(str::trim)
    .filter(|line| !line.is_empty() && !line.starts_with("--"));
  let Some(first) = statements.next() else {
    return false;
  };
  statements.next().is_none() && is_foreign_keys_statement(first)
}

/// True when a SQLite migration sets `PRAGMA foreign_keys` anywhere, which is
/// how a generated table rebuild announces itself.
pub(super) fn manages_pragmas(sql: &str, dialect: Dialect) -> bool {
  match dialect {
    Dialect::Postgres => false,
    Dialect::Sqlite => sql.lines().any(is_foreign_keys_statement),
  }
}

/// `ON` / `OFF` for a pragma value.
fn on_off(enabled: bool) -> &'static str {
  if enabled {
    "ON"
  } else {
    "OFF"
  }
}

/// The batch that puts both pragmas back the way the caller had them.
pub(super) fn restore_sql(foreign_keys: bool, legacy_alter_table: bool) -> String {
  format!(
    "PRAGMA foreign_keys = {}; PRAGMA legacy_alter_table = {};",
    on_off(foreign_keys),
    on_off(legacy_alter_table)
  )
}

/// Turns a single-integer pragma row into a flag.
pub(super) fn flag(rows: &[PragmaInt]) -> bool {
  rows.first().is_some_and(|row| row.value != 0)
}

/// Reads both pragmas and, when foreign keys are enforced, switches them off.
/// Call before `BEGIN`: inside a transaction the write would do nothing.
pub(super) async fn arm_pragmas(
  conn: &impl DbConnection,
  sql: &str,
  dialect: Dialect,
) -> Result<PragmaGuard, MigrateError> {
  if !manages_pragmas(sql, dialect) {
    return Ok(PragmaGuard::Untouched);
  }
  let foreign_keys = flag(&read_pragma(conn, READ_FOREIGN_KEYS).await?);
  let legacy_alter_table = flag(&read_pragma(conn, READ_LEGACY_ALTER_TABLE).await?);
  if foreign_keys {
    conn
      .execute_batch(DISABLE_FOREIGN_KEYS)
      .await
      .map_err(|e| map_db(&e))?;
  }
  Ok(PragmaGuard::Armed {
    foreign_keys,
    legacy_alter_table,
  })
}

async fn read_pragma(conn: &impl DbConnection, sql: &str) -> Result<Vec<PragmaInt>, MigrateError> {
  conn
    .query_map::<PragmaInt>(sql, vec![])
    .await
    .map_err(|e| map_db(&e))
}

/// Verifies the migration left no dangling reference behind. Call inside the
/// transaction, just before the commit, so a violation rolls everything back.
/// Only meaningful when foreign keys were enforced to begin with.
pub(super) async fn check_foreign_keys(
  conn: &impl DbConnection,
  guard: PragmaGuard,
  file: &str,
) -> Result<(), MigrateError> {
  if !enforced_before(guard) {
    return Ok(());
  }
  let rows = read_pragma(conn, COUNT_FOREIGN_KEY_VIOLATIONS).await?;
  let count = rows.first().map_or(0, |row| row.value);
  if count == 0 {
    return Ok(());
  }
  Err(MigrateError::ForeignKeyViolation {
    file: file.to_owned(),
    count,
  })
}

/// Whether the connection enforced foreign keys before the migration started.
pub(super) fn enforced_before(guard: PragmaGuard) -> bool {
  match guard {
    PragmaGuard::Untouched => false,
    PragmaGuard::Armed { foreign_keys, .. } => foreign_keys,
  }
}

/// Puts both pragmas back after the transaction ended, whether it committed or
/// rolled back. A restore that fails is folded into `outcome` so neither
/// failure is lost.
pub(super) async fn restore_pragmas(
  conn: &impl DbConnection,
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
  match conn
    .execute_batch(&restore_sql(foreign_keys, legacy_alter_table))
    .await
  {
    Ok(()) => outcome,
    Err(restore_err) => Err(restore_failure(outcome, &restore_err.to_string())),
  }
}

/// Reports a failed restore without losing what the migration itself reported:
/// the message carries both, and the migration's own error stays whole as the
/// [`MigrateError::PragmaRestore`] source so a caller can still match on it.
pub(super) fn restore_failure(
  outcome: Result<(), MigrateError>,
  restore_err: &str,
) -> MigrateError {
  let Err(prior) = outcome else {
    return MigrateError::PragmaRestore {
      message: format!("restoring SQLite pragmas failed: {restore_err}"),
      source: None,
    };
  };
  MigrateError::PragmaRestore {
    message: format!("{prior}; restoring SQLite pragmas also failed: {restore_err}"),
    source: Some(Box::new(prior)),
  }
}
