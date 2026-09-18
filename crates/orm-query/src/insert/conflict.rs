//! The conflict policy an [`super::InsertBuilder`] carries, and the legacy
//! Postgres rendering of `or_replace()`.

use toolu_orm_core::alias::quote_ident;

use super::on_conflict::OnConflict;

/// How an `INSERT` reacts to a uniqueness conflict.
///
/// One field on the builder holds exactly one of these, so a statement can
/// never carry two contradictory conflict policies: `or_replace()`,
/// `or_ignore()` and `on_conflict(...)` overwrite each other and the last
/// call wins.
pub(super) enum ConflictMode {
  None,
  Replace,
  Ignore,
  /// An explicit `ON CONFLICT (…) DO …`, rendered the same on both dialects.
  Clause(OnConflict),
}

/// Appends the Postgres `ON CONFLICT (…) DO UPDATE SET … = EXCLUDED.…` that
/// approximates SQLite's `INSERT OR REPLACE`.
///
/// The target is the caller's `conflict_columns(&[…])`, falling back to the
/// first inserted column; every other inserted column is copied from
/// `EXCLUDED`. This is the pre-#108 shorthand, kept verbatim for
/// compatibility — [`OnConflict`] is the form that says what it means.
pub(super) fn push_legacy_postgres_replace(
  sql: &mut String,
  columns: &[String],
  conflict_cols: &[String],
) {
  let target: Vec<String> = if conflict_cols.is_empty() {
    columns.first().cloned().into_iter().collect()
  } else {
    conflict_cols.to_vec()
  };

  let target_sql: Vec<String> = target.iter().map(|c| quote_ident(c)).collect();
  let update_cols: Vec<String> = columns
    .iter()
    .filter(|c| !target.iter().any(|t| t == *c))
    .map(|c| format!("{} = EXCLUDED.{}", quote_ident(c), quote_ident(c)))
    .collect();

  sql.push_str(&format!(" ON CONFLICT ({}) DO ", target_sql.join(", ")));

  if update_cols.is_empty() {
    sql.push_str("NOTHING");
  } else {
    sql.push_str(&format!("UPDATE SET {}", update_cols.join(", ")));
  }
}
