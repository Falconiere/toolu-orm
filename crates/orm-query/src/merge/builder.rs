//! Source batch and explicit branch policies for native MERGE.

use toolu_orm_connection::{DbConnection, DbError};
use toolu_orm_core::{alias::TableRef, query_column::ColumnRef, value::Value};

/// Action for every target row whose keys equal a source row's keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Matched {
  /// Assign every supplied non-key column from the source.
  Update,
  /// Leave matching target rows unchanged.
  DoNothing,
}

/// Action for a source row with no matching target key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotMatched {
  /// Insert the supplied columns; omitted columns use database defaults.
  Insert,
  /// Discard the unmatched source row.
  DoNothing,
}

/// One explicit native MERGE on Lance or PostgreSQL 15+.
///
/// Both policies must be specified. Keys are equality predicates, not unique
/// constraints: one source row updates **all** matching duplicate target rows.
/// Concurrent writers can insert duplicate absent keys unless the database
/// enforces uniqueness. SQLite refuses this operation; use its separate
/// [`OnConflict`](crate::insert::OnConflict) API when appropriate.
///
/// Keys accept non-null integer, text or boolean values, with one variant per
/// key column. Repeated exact source tuples are rejected. Callers must use key
/// values matching target types and equality/collation semantics; coercion and
/// custom collations that collapse distinct values are outside this contract.
#[derive(Debug, Clone)]
pub struct MergeBuilder {
  pub(super) table: TableRef,
  pub(super) columns: Vec<String>,
  pub(super) keys: Vec<String>,
  pub(super) rows: Vec<Vec<Value>>,
  pub(super) matched: Option<Matched>,
  pub(super) not_matched: Option<NotMatched>,
}

impl MergeBuilder {
  /// Start a merge for a literal table name.
  pub fn new(table: &str) -> Self {
    Self::into_table(table)
  }

  /// Start a merge for a possibly schema-qualified relation.
  /// Caller aliases are replaced with internal aliases; functions are refused.
  pub fn into_table(table: impl Into<TableRef>) -> Self {
    Self {
      table: table.into(),
      columns: Vec::new(),
      keys: Vec::new(),
      rows: Vec::new(),
      matched: None,
      not_matched: None,
    }
  }

  /// Declare supplied target columns in source-row order, replacing prior names.
  #[must_use]
  pub fn columns(mut self, columns: &[&dyn ColumnRef]) -> Self {
    self.columns = columns.iter().map(|c| c.name().to_owned()).collect();
    self
  }

  /// Declare equality keys; each must appear once in the supplied columns.
  #[must_use]
  pub fn keys(mut self, keys: &[&dyn ColumnRef]) -> Self {
    self.keys = keys.iter().map(|c| c.name().to_owned()).collect();
    self
  }

  /// Append one row, with exactly one value per declared column.
  #[must_use]
  pub fn row(mut self, values: Vec<Value>) -> Self {
    self.rows.push(values);
    self
  }

  /// Explicitly choose the action for matching rows.
  #[must_use]
  pub fn when_matched(mut self, policy: Matched) -> Self {
    self.matched = Some(policy);
    self
  }

  /// Explicitly choose the action for absent keys.
  #[must_use]
  pub fn when_not_matched(mut self, policy: NotMatched) -> Self {
    self.not_matched = Some(policy);
    self
  }

  /// Execute one statement and return the engine's affected-row count.
  ///
  /// # Errors
  /// Refuses SQLite and malformed batches before execution. Database and codec
  /// errors propagate without a count. This does not provide multi-statement
  /// transactions or uniqueness guarantees.
  pub async fn execute_on(&self, conn: &impl DbConnection) -> Result<u64, DbError> {
    let (sql, values) = self.to_sql_for(conn.dialect())?;
    conn.execute_sql(&sql, values).await
  }
}
