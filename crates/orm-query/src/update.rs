//! UPDATE query builder with SET clause and filter support.

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::{BoundParams, Expr, Scalar};
use toolu_orm_core::query_column::Column;
use toolu_orm_core::value::Value;

use crate::where_clause::{append_where_for, cfg_single_backend, impl_filter};

cfg_single_backend! {
  use crate::exec_helpers::impl_execute;
}

// ── UpdateBuilder ─────────────────────────────────────────────────────────────

pub struct UpdateBuilder {
  table: String,
  /// Assigned column name and the scalar it is set to, in call order.
  sets: Vec<(String, Scalar)>,
  filters: Vec<Expr>,
}

impl_filter!(UpdateBuilder);

impl UpdateBuilder {
  pub fn new(table: &str) -> Self {
    Self {
      table: table.to_owned(),
      sets: Vec::new(),
      filters: Vec::new(),
    }
  }

  /// `"<column>" = ?N` — one bound value.
  pub fn set<T>(mut self, col: &Column<T>, val: impl Into<Value>) -> Self {
    self.sets.push((col.name.to_owned(), Scalar::bind(val)));
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

  pub fn to_sql_for(&self, dialect: Dialect) -> (String, Vec<Value>) {
    let mut sql = format!(r#"UPDATE "{}" SET "#, self.table);
    let mut params = BoundParams::new();

    let mut parts: Vec<String> = Vec::with_capacity(self.sets.len());
    for (column, value) in &self.sets {
      let fragment = value.render_into(&mut params, dialect);
      parts.push(format!(r#""{column}" = {fragment}"#));
    }
    sql.push_str(&parts.join(", "));

    append_where_for(&self.filters, &mut sql, &mut params, dialect);

    (sql, params.into_values())
  }

  pub fn to_sql(&self) -> (String, Vec<Value>) {
    self.to_sql_for(Dialect::CURRENT)
  }
}

cfg_single_backend! {
  impl_execute!(UpdateBuilder, "UPDATE");
}
