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

/// SELECT used by [`super::store::get_applied_migrations`] and its blocking
/// twin. It carries `hash` as well as `name` because a runner validates the
/// history it is about to skip, and one statement serves both readings.
pub(super) const SELECT_APPLIED_MIGRATIONS: &str = "SELECT name, hash FROM _migrations ORDER BY id";

pub(super) const BEGIN: &str = "BEGIN";
pub(super) const COMMIT: &str = "COMMIT";
pub(super) const ROLLBACK: &str = "ROLLBACK";

/// Reads SQLite's current foreign-key enforcement as a single `0`/`1` row.
pub(super) const READ_FOREIGN_KEYS: &str = "PRAGMA foreign_keys";
/// Suspends foreign keys. Only has an effect outside a transaction.
pub(super) const DISABLE_FOREIGN_KEYS: &str = "PRAGMA foreign_keys = OFF";
/// Reads the caller's `legacy_alter_table` setting, which a rebuild flips
/// across its final rename and the runner puts back.
pub(super) const READ_LEGACY_ALTER_TABLE: &str = "PRAGMA legacy_alter_table";
/// Counts the rows that violate a foreign key, database-wide.
pub(super) const COUNT_FOREIGN_KEY_VIOLATIONS: &str =
  "SELECT count(*) FROM pragma_foreign_key_check";
