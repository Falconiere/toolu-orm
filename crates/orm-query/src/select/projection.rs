//! The `SELECT` projection list: plain columns, raw fragments, and scalars.

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::Scalar;
use toolu_orm_core::value::Value;

use super::SelectBuilder;

impl SelectBuilder {
  /// `<expr> AS "<alias>"` from SQL text that binds nothing — an FTS5
  /// `bm25(...)`, a `vec0` distance, or any literal.
  ///
  /// [`SelectBuilder::column_scalar`] is the form that carries parameters.
  /// Both push onto the same list, so projections render in call order
  /// whichever one produced them.
  pub fn column_expr(mut self, expr: &str, alias: &str) -> Self {
    self
      .column_exprs
      .push((Scalar::sql(expr), alias.to_owned()));
    self
  }

  /// `<scalar> AS "<alias>"` — a computed output that may bind values.
  ///
  /// Its placeholders are numbered before the `WHERE` clause's, because the
  /// select list is rendered first; the returned parameter vector is in that
  /// same order.
  pub fn column_scalar(mut self, expr: Scalar, alias: &str) -> Self {
    self.column_exprs.push((expr, alias.to_owned()));
    self
  }

  /// Plain columns first, then the aliased expressions.
  ///
  /// Only the non-empty halves are joined, so a builder carrying just one of
  /// the two gains no stray comma. The expressions used to be dropped unless
  /// the builder came from [`SelectBuilder::raw`], which silently discarded an
  /// FTS5 `bm25(...)` projection on an ordinary table.
  pub(super) fn build_select_list(&self, params: &mut Vec<Value>, dialect: Dialect) -> String {
    let mut parts: Vec<String> = self.columns.iter().map(|c| format!(r#""{c}""#)).collect();
    for (expr, alias) in &self.column_exprs {
      let start = params.len() + 1;
      let (fragment, expr_params) = expr.to_sql_fragment_for(start, dialect);
      params.extend(expr_params);
      parts.push(format!(r#"{fragment} AS "{alias}""#));
    }
    parts.join(", ")
  }
}
