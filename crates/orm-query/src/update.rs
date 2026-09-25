//! UPDATE query builder with SET clause, filter support and `RETURNING`.

use toolu_orm_core::alias::quote_ident;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::{BoundParams, Expr, Scalar};
use toolu_orm_core::query_column::{tag_column_bind, Column};
use toolu_orm_core::value::Value;

use crate::where_clause::{append_returning, append_where_for, cfg_single_backend, impl_filter};

cfg_single_backend! {
  use crate::exec_helpers::{impl_execute, impl_returning_fetch};
}

// ── UpdateBuilder ─────────────────────────────────────────────────────────────

/// Typed UPDATE query builder.
pub struct UpdateBuilder {
  table: String,
  /// Assigned column name and the scalar it is set to, in call order.
  sets: Vec<(String, Scalar)>,
  filters: Vec<Expr>,
  /// Column names projected by `RETURNING`, in call order.
  returning: Vec<String>,
}

impl_filter!(UpdateBuilder);

impl UpdateBuilder {
  /// Start an UPDATE for a table name.
  pub fn new(table: &str) -> Self {
    Self {
      table: table.to_owned(),
      sets: Vec::new(),
      filters: Vec::new(),
      returning: Vec::new(),
    }
  }

  /// The target table, which is what `QueryError::NotFound` carries.
  pub fn table_name(&self) -> &str {
    &self.table
  }

  /// `"<column>" = ?N` — one bound value.
  pub fn set<T: 'static>(mut self, col: &Column<T>, val: impl Into<Value>) -> Self {
    self.sets.push((
      col.name.to_owned(),
      Scalar::bind(tag_column_bind::<T>(val.into())),
    ));
    self
  }

  /// `"<column>" = <expr>` from SQL text that binds nothing.
  ///
  /// [`UpdateBuilder::set_scalar`] is the form that carries parameters and
  /// composes: `access_count = access_count + 1` is
  /// `set_scalar(&HITS, Scalar::col(&HITS) + Scalar::bind(1))`.
  pub fn set_expr<T>(mut self, col: &Column<T>, expr: &str) -> Self {
    self.sets.push((col.name.to_owned(), Scalar::sql(expr)));
    self
  }

  /// `"<column>" = <scalar>` — a computed assignment that may bind values.
  ///
  /// Its placeholders are numbered before the `WHERE` clause's, because `SET`
  /// is rendered first.
  pub fn set_scalar<T>(mut self, col: &Column<T>, expr: Scalar) -> Self {
    self.sets.push((col.name.to_owned(), expr));
    self
  }

  /// Appends one column to `RETURNING "a", "b"`, in call order.
  ///
  /// Rendered last and unqualified, which both engines accept, and it binds
  /// nothing. Read the projected rows with `fetch_one` / `fetch_optional` /
  /// `fetch_all` rather than `execute`: rusqlite refuses to `execute` a
  /// row-producing statement. A filter that matches nothing produces no row,
  /// so `fetch_optional` returns `None` and `fetch_one` is
  /// `QueryError::NotFound`.
  pub fn returning<T>(mut self, col: &Column<T>) -> Self {
    self.returning.push(col.name.to_owned());
    self
  }

  /// Render this UPDATE for an explicit dialect.
  pub fn to_sql_for(&self, dialect: Dialect) -> (String, Vec<Value>) {
    let mut sql = format!("UPDATE {} SET ", quote_ident(&self.table));
    let mut params = BoundParams::new();

    let mut parts: Vec<String> = Vec::with_capacity(self.sets.len());
    for (column, value) in &self.sets {
      let fragment = value.render_into(&mut params, dialect);
      parts.push(format!("{} = {fragment}", quote_ident(column)));
    }
    sql.push_str(&parts.join(", "));

    append_where_for(&self.filters, &mut sql, &mut params, dialect);
    append_returning(&self.returning, &mut sql);

    (sql, params.into_values())
  }

  /// Render this UPDATE for the compile-time default dialect.
  pub fn to_sql(&self) -> (String, Vec<Value>) {
    self.to_sql_for(Dialect::CURRENT)
  }
}

cfg_single_backend! {
  impl_execute!(UpdateBuilder, "UPDATE");
  impl_returning_fetch!(UpdateBuilder, "UPDATE");
}
