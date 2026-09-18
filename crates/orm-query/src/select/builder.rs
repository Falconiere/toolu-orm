//! Core [`SelectBuilder`] struct with SQL generation methods.
//!
//! # Public API
//!
//! - [`SelectBuilder`] — fluent builder for SELECT queries
//!
//! # Usage
//!
//! ```ignore
//! let (sql, params) = SelectBuilder::new("users")
//!     .columns_raw(&["id", "name"])
//!     .to_sql();
//! ```

use toolu_orm_core::alias::TableRef;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::{Expr, JoinCondition, OrderBy, Scalar};
use toolu_orm_core::query_column::ColumnRef;
use toolu_orm_core::value::Value;

use crate::where_clause::{append_where_for, impl_filter};

use super::join_clause::JoinClause;

// ── SelectBuilder ─────────────────────────────────────────────────────────────

pub struct SelectBuilder {
  pub(super) table: TableRef,
  /// Select-list items, already rendered: `"id"` or `"users"."id"`.
  pub(super) columns: Vec<String>,
  pub(super) filters: Vec<Expr>,
  pub(super) joins: Vec<JoinClause>,
  pub(super) order_bys: Vec<OrderBy>,
  pub(super) limit_val: Option<i64>,
  pub(super) offset_val: Option<i64>,
  pub(super) column_exprs: Vec<(Scalar, String)>,
  pub(super) is_raw: bool,
  pub(super) knn_applied: bool,
}

impl_filter!(SelectBuilder);

impl SelectBuilder {
  pub fn new(table: &str) -> Self {
    Self::from_table(table)
  }

  /// [`Self::new`] for a table that may carry an alias.
  ///
  /// `TableRef::aliased("memories", "old")` renders `FROM "memories" AS "old"`,
  /// which is what a self-join needs; `&str` and `&TableRef` also convert.
  pub fn from_table(table: impl Into<TableRef>) -> Self {
    Self {
      table: table.into(),
      columns: Vec::new(),
      filters: Vec::new(),
      joins: Vec::new(),
      order_bys: Vec::new(),
      limit_val: None,
      offset_val: None,
      column_exprs: Vec::new(),
      is_raw: false,
      knn_applied: false,
    }
  }

  pub fn raw() -> Self {
    Self {
      is_raw: true,
      ..Self::from_table("")
    }
  }

  pub fn columns_raw(mut self, cols: &[&str]) -> Self {
    self.columns = cols.iter().map(|c| format!(r#""{c}""#)).collect();
    self
  }

  /// Typed columns projected under their bare names: `"id", "email"`.
  ///
  /// Enough for a single-table query. After a join, where two tables may both
  /// have an `id`, use [`Self::columns_qualified`] — a bare name is ambiguous
  /// there and the database rejects the statement.
  pub fn columns_typed(mut self, cols: &[&dyn ColumnRef]) -> Self {
    self.columns = cols.iter().map(|c| format!(r#""{}""#, c.name())).collect();
    self
  }

  /// Appends `INNER JOIN <table> ON <condition>`.
  ///
  /// `table` may be a `&str`, a [`TableRef`] or `&TableRef`; `on` may be a
  /// [`JoinCondition`] or a plain [`Expr`].
  pub fn join(mut self, table: impl Into<TableRef>, on: impl Into<JoinCondition>) -> Self {
    self.joins.push(JoinClause::inner(table.into(), on.into()));
    self
  }

  /// Appends `LEFT JOIN <table> ON <condition>`; see [`Self::join`].
  ///
  /// A predicate belongs in the `ON` clause, not in `filter`: moving it to
  /// `WHERE` drops the unmatched rows a `LEFT JOIN` exists to keep.
  pub fn left_join(mut self, table: impl Into<TableRef>, on: impl Into<JoinCondition>) -> Self {
    self.joins.push(JoinClause::left(table.into(), on.into()));
    self
  }

  pub fn limit(mut self, n: i64) -> Self {
    self.limit_val = Some(n);
    self
  }

  pub fn offset(mut self, n: i64) -> Self {
    self.offset_val = Some(n);
    self
  }

  pub fn to_sql_for(&self, dialect: Dialect) -> (String, Vec<Value>) {
    self.to_sql_with_limit(dialect, self.limit_val)
  }

  /// [`Self::to_sql_for`] with `limit` standing in for [`Self::limit`].
  ///
  /// Everything else — select list, joins, `WHERE`, `ORDER BY`, `OFFSET` — is
  /// rendered exactly as `to_sql_for` renders it. The row bound of
  /// [`Self::to_first_row_sql_for`] is the only caller that passes something
  /// other than `self.limit_val`.
  pub(super) fn to_sql_with_limit(
    &self,
    dialect: Dialect,
    limit: Option<i64>,
  ) -> (String, Vec<Value>) {
    let mut sql = String::new();
    let mut params: Vec<Value> = Vec::new();

    sql.push_str("SELECT ");
    let select_list = self.build_select_list(&mut params, dialect);
    sql.push_str(&select_list);

    if !self.is_raw {
      sql.push_str(&format!(" FROM {}", self.table.to_sql()));
      self.append_joins(&mut sql, &mut params, dialect);
    }

    append_where_for(&self.filters, &mut sql, &mut params, dialect);
    self.append_order_by(&mut sql, &mut params, dialect);
    self.append_limit_offset_for(&mut sql, &mut params, dialect, limit);

    (sql, params)
  }

  pub fn to_sql(&self) -> (String, Vec<Value>) {
    self.to_sql_for(Dialect::CURRENT)
  }

  pub fn to_count_sql_for(&self, dialect: Dialect) -> (String, Vec<Value>) {
    let mut sql = String::new();
    let mut params: Vec<Value> = Vec::new();

    sql.push_str(&format!("SELECT COUNT(*) FROM {}", self.table.to_sql()));
    self.append_joins(&mut sql, &mut params, dialect);
    append_where_for(&self.filters, &mut sql, &mut params, dialect);

    (sql, params)
  }

  pub fn to_count_sql(&self) -> (String, Vec<Value>) {
    self.to_count_sql_for(Dialect::CURRENT)
  }

  pub fn to_exists_sql_for(&self, dialect: Dialect) -> (String, Vec<Value>) {
    let mut inner = String::new();
    let mut params: Vec<Value> = Vec::new();

    inner.push_str(&format!("SELECT 1 FROM {}", self.table.to_sql()));
    self.append_joins(&mut inner, &mut params, dialect);
    append_where_for(&self.filters, &mut inner, &mut params, dialect);

    (format!("SELECT EXISTS({inner})"), params)
  }

  pub fn to_exists_sql(&self) -> (String, Vec<Value>) {
    self.to_exists_sql_for(Dialect::CURRENT)
  }

  /// The base table name, never an alias.
  pub fn table_name(&self) -> &str {
    self.table.table()
  }

  /// The table this builder selects from, alias included.
  pub fn table_ref(&self) -> &TableRef {
    &self.table
  }
}
