//! The FTS5 markup functions: `snippet()` and `highlight()`, plus the column
//! index they address a column by.

use crate::dialect::Dialect;
use crate::error::DbCoreError;

use super::super::literal::{quoted_string, quoted_table, require_sqlite};
use super::call::Fts5Fn;

const SNIPPET: &str = "snippet";
const HIGHLIGHT: &str = "highlight";
const COLUMN_INDEX: &str = "column_index";

/// Arguments of `snippet(<table>, <column>, <open>, <close>, <ellipsis>, <tokens>)`.
#[derive(Debug, Clone, Copy)]
pub struct Snippet<'a> {
  /// 0-based FTS5 column number; `-1` means every column. Build it from a
  /// generated `ALL_COLUMNS` with [`column_index`].
  pub column_index: i32,
  /// Inserted before each matched term.
  pub open: &'a str,
  /// Inserted after each matched term.
  pub close: &'a str,
  /// Marks text trimmed from either end of the snippet.
  pub ellipsis: &'a str,
  /// Snippet length in tokens; FTS5 requires `1..=64`.
  pub tokens: i32,
}

/// Arguments of `highlight(<table>, <column>, <open>, <close>)`.
#[derive(Debug, Clone, Copy)]
pub struct Highlight<'a> {
  /// 0-based FTS5 column number; `-1` means every column. Build it from a
  /// generated `ALL_COLUMNS` with [`column_index`].
  pub column_index: i32,
  /// Inserted before each matched term.
  pub open: &'a str,
  /// Inserted after each matched term.
  pub close: &'a str,
}

/// `snippet(…)` for an explicit dialect: the column's text around the match,
/// trimmed to `tokens` tokens, with each matched term wrapped in the tags.
///
/// # Errors
///
/// - [`DbCoreError::Fts5UnsupportedDialect`] for [`Dialect::Postgres`].
/// - [`DbCoreError::Fts5InvalidArgument`] when `table` is not a plain
///   identifier, `column_index` is below `-1`, or `tokens` is outside
///   `1..=64`. FTS5 documents that range but silently clamps out-of-range
///   values instead of complaining, so refusing here is the only way a caller
///   learns the number did not mean what they wrote.
///
/// An index past the table's last column cannot be caught here — the column
/// count is not known at this point — and surfaces as a driver error when the
/// statement is prepared.
pub fn snippet_for(
  dialect: Dialect,
  table: &str,
  spec: &Snippet<'_>,
) -> Result<Fts5Fn, DbCoreError> {
  require_sqlite(SNIPPET, dialect)?;
  let table = quoted_table(SNIPPET, table)?;
  validate_column_index(SNIPPET, spec.column_index)?;
  validate_tokens(spec.tokens)?;

  Ok(Fts5Fn::new(format!(
    "{SNIPPET}({table}, {}, {}, {}, {}, {})",
    spec.column_index,
    quoted_string(spec.open),
    quoted_string(spec.close),
    quoted_string(spec.ellipsis),
    spec.tokens
  )))
}

/// [`snippet_for`] against [`Dialect::CURRENT`].
///
/// # Errors
///
/// See [`snippet_for`].
pub fn snippet(table: &str, spec: &Snippet<'_>) -> Result<Fts5Fn, DbCoreError> {
  snippet_for(Dialect::CURRENT, table, spec)
}

/// `highlight(…)` for an explicit dialect: the column's whole text with each
/// matched term wrapped in the tags.
///
/// # Errors
///
/// - [`DbCoreError::Fts5UnsupportedDialect`] for [`Dialect::Postgres`].
/// - [`DbCoreError::Fts5InvalidArgument`] when `table` is not a plain
///   identifier or `column_index` is below `-1`.
pub fn highlight_for(
  dialect: Dialect,
  table: &str,
  spec: &Highlight<'_>,
) -> Result<Fts5Fn, DbCoreError> {
  require_sqlite(HIGHLIGHT, dialect)?;
  let table = quoted_table(HIGHLIGHT, table)?;
  validate_column_index(HIGHLIGHT, spec.column_index)?;

  Ok(Fts5Fn::new(format!(
    "{HIGHLIGHT}({table}, {}, {}, {})",
    spec.column_index,
    quoted_string(spec.open),
    quoted_string(spec.close)
  )))
}

/// [`highlight_for`] against [`Dialect::CURRENT`].
///
/// # Errors
///
/// See [`highlight_for`].
pub fn highlight(table: &str, spec: &Highlight<'_>) -> Result<Fts5Fn, DbCoreError> {
  highlight_for(Dialect::CURRENT, table, spec)
}

/// The 0-based position of `name` in `columns`, for [`Snippet`] and
/// [`Highlight`].
///
/// Pass the `ALL_COLUMNS` constant `#[fts5_table]` generates, so the index
/// follows the declaration order the DDL used:
///
/// ```
/// use toolu_orm_core::fts5;
///
/// # fn main() -> Result<(), toolu_orm_core::error::DbCoreError> {
/// let columns = ["memory_id", "body", "tags"];
/// assert_eq!(fts5::column_index(&columns, "body")?, 1);
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// [`DbCoreError::Fts5InvalidArgument`] when `name` is absent, listing the
/// columns that are present.
pub fn column_index(columns: &[&str], name: &str) -> Result<i32, DbCoreError> {
  let position = columns
    .iter()
    .position(|candidate| *candidate == name)
    .ok_or_else(|| DbCoreError::Fts5InvalidArgument {
      function: COLUMN_INDEX.to_owned(),
      reason: format!("no column {name:?}; the table declares {columns:?}"),
    })?;
  i32::try_from(position).map_err(|source| DbCoreError::Fts5InvalidArgument {
    function: COLUMN_INDEX.to_owned(),
    reason: format!(
      "column {name:?} is at position {position}, past what FTS5 can address: {source}"
    ),
  })
}

fn validate_column_index(function: &str, index: i32) -> Result<(), DbCoreError> {
  if index >= -1 {
    return Ok(());
  }
  Err(DbCoreError::Fts5InvalidArgument {
    function: function.to_owned(),
    reason: format!("column index {index} is below -1; use 0.. for one column, -1 for all"),
  })
}

fn validate_tokens(tokens: i32) -> Result<(), DbCoreError> {
  if (1..=64).contains(&tokens) {
    return Ok(());
  }
  Err(DbCoreError::Fts5InvalidArgument {
    function: SNIPPET.to_owned(),
    reason: format!("token count {tokens} is outside the 1..=64 FTS5 accepts"),
  })
}
