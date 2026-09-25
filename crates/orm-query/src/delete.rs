//! DELETE query builder with dialect-aware SQL generation and `RETURNING`.

use toolu_orm_core::alias::quote_ident;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::{BoundParams, Expr};
use toolu_orm_core::query_column::Column;
use toolu_orm_core::value::Value;

use crate::where_clause::{append_returning, append_where_for, cfg_single_backend, impl_filter};

cfg_single_backend! {
  use crate::exec_helpers::{impl_execute, impl_returning_fetch};
}

// ── DeleteBuilder ─────────────────────────────────────────────────────────────

/// Typed DELETE query builder.
pub struct DeleteBuilder {
  table: String,
  filters: Vec<Expr>,
  /// Column names projected by `RETURNING`, in call order.
  returning: Vec<String>,
}

impl_filter!(DeleteBuilder);

impl DeleteBuilder {
  /// Start a DELETE for a table name.
  pub fn new(table: &str) -> Self {
    Self {
      table: table.to_owned(),
      filters: Vec::new(),
      returning: Vec::new(),
    }
  }

  /// The target table, which is what `QueryError::NotFound` carries.
  pub fn table_name(&self) -> &str {
    &self.table
  }

  /// Appends one column to `RETURNING "a", "b"`, in call order.
  ///
  /// Rendered last and unqualified, which both engines accept, and it binds
  /// nothing. Read the projected rows with `fetch_one` / `fetch_optional` /
  /// `fetch_all` rather than `execute`: rusqlite refuses to `execute` a
  /// row-producing statement. A filter that matches nothing produces no row,
  /// so `fetch_optional` returns `None` and `fetch_one` is
  /// `QueryError::NotFound`. The projected values are the removed row.
  pub fn returning<T>(mut self, col: &Column<T>) -> Self {
    self.returning.push(col.name.to_owned());
    self
  }

  /// Render this DELETE for an explicit dialect.
  pub fn to_sql_for(&self, dialect: Dialect) -> (String, Vec<Value>) {
    let mut sql = String::new();
    let mut params = BoundParams::new();

    sql.push_str(&format!("DELETE FROM {}", quote_ident(&self.table)));
    append_where_for(&self.filters, &mut sql, &mut params, dialect);
    append_returning(&self.returning, &mut sql);

    (sql, params.into_values())
  }

  /// Render this DELETE for the compile-time default dialect.
  pub fn to_sql(&self) -> (String, Vec<Value>) {
    self.to_sql_for(Dialect::CURRENT)
  }
}

cfg_single_backend! {
  impl_execute!(DeleteBuilder, "DELETE");
  impl_returning_fetch!(DeleteBuilder, "DELETE");
}
