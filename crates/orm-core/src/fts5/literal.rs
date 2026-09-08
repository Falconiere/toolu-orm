//! Rendering the pieces an FTS5 call is made of: the table identifier, the
//! quoted string arguments, and the `f64` column weights.
//!
//! FTS5 rejects bound parameters as auxiliary-function arguments — `bm25(t, ?)`
//! is a syntax error — so every argument here becomes literal SQL text. That is
//! the whole reason this module exists: the formatting happens once, under
//! validation, instead of in a `format!` at each call site.

use crate::dialect::Dialect;
use crate::error::DbCoreError;

/// Refuses anything but SQLite.
///
/// FTS5 is a SQLite module; Postgres full-text search is `@@` / `to_tsquery` /
/// `ts_rank`, a different enough model that translating between them silently
/// would be wrong. Rejecting here means no Postgres SQL containing an FTS5
/// call can be built at all.
pub(crate) fn require_sqlite(function: &str, dialect: Dialect) -> Result<(), DbCoreError> {
  match dialect {
    Dialect::Sqlite => Ok(()),
    Dialect::Postgres => Err(DbCoreError::Fts5UnsupportedDialect {
      function: function.to_owned(),
      dialect: dialect.as_str(),
    }),
  }
}

/// A validated FTS5 table reference, double-quoted.
///
/// The name is *validated* rather than escaped: it addresses an FTS5 table, so
/// anything that is not a plain identifier is a caller mistake, not a value to
/// quote. Nothing outside `[A-Za-z_][A-Za-z0-9_]*` is ever emitted, which
/// leaves no route for a quote, a semicolon or a comment to reach the SQL.
///
/// A table *alias* is not accepted by FTS5 in this position (`bm25(f, …)` with
/// `memory_fts f` fails with `no such column: f`), so callers pass the table's
/// own name.
pub(crate) fn quoted_table(function: &str, table: &str) -> Result<String, DbCoreError> {
  let invalid = |reason: &str| DbCoreError::Fts5InvalidArgument {
    function: function.to_owned(),
    reason: reason.to_owned(),
  };

  let mut chars = table.chars();
  let first = chars
    .next()
    .ok_or_else(|| invalid("the table name is empty"))?;
  if !(first.is_ascii_alphabetic() || first == '_') {
    return Err(invalid(&format!(
      "table name {table:?} must start with an ASCII letter or underscore"
    )));
  }
  if !chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
    return Err(invalid(&format!(
      "table name {table:?} must contain only ASCII letters, digits and underscores"
    )));
  }
  Ok(format!("\"{table}\""))
}

/// A single-quoted SQL string literal with its embedded quotes doubled, the
/// same escaping [`crate::fts5::Fts5Options`] uses on the DDL side.
pub(super) fn quoted_string(text: &str) -> String {
  format!("'{}'", text.replace('\'', "''"))
}

/// A `f64` as SQL float text.
///
/// `Debug` rather than `Display`: Display renders `0.0` as `0`, an integer
/// literal, while Debug keeps `0.0`, `1.5` and `1e-10` — all of which SQLite
/// reads as floats. Non-finite values are refused here too, so a future
/// `pub(super)` caller that skips the BM25 weight check still cannot emit
/// `inf` / `NaN` into SQL.
///
/// # Errors
///
/// [`DbCoreError::Fts5InvalidArgument`] when `value` is NaN or infinite.
pub(super) fn float_literal(function: &str, value: f64) -> Result<String, DbCoreError> {
  if !value.is_finite() {
    return Err(DbCoreError::Fts5InvalidArgument {
      function: function.to_owned(),
      reason: format!("{value} is not a finite float; refusing to emit it as SQL"),
    });
  }
  Ok(format!("{value:?}"))
}
