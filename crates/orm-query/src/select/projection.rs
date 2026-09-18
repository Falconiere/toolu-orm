//! The `SELECT` projection list: plain columns, raw fragments, and scalars.

use toolu_orm_core::alias::QualifiedColumn;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::{BoundParams, Scalar};

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

  /// Typed columns projected qualified: `"users"."id", "old"."id"`.
  ///
  /// Takes plain and aliased columns in one slice, so a joined query can name
  /// exactly which relation each item comes from. Pair it with
  /// [`SelectBuilder::column_as`] when two items would otherwise share an
  /// output name.
  pub fn columns_qualified(mut self, cols: &[&dyn QualifiedColumn]) -> Self {
    self.columns = cols.iter().map(|c| c.qualified()).collect();
    self
  }

  /// `"qualifier"."column" AS "<alias>"` — the qualified twin of
  /// [`SelectBuilder::column_expr`].
  ///
  /// The output alias is what tells two same-named columns apart in the result
  /// — `"c"."id" AS "c_id"` next to `"f"."id" AS "f_id"`.
  pub fn column_as(self, col: &dyn QualifiedColumn, out_alias: &str) -> Self {
    self.column_scalar(Scalar::sql(col.qualified()), out_alias)
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
  /// The plain half arrives already rendered, so a qualified item and a bare
  /// one look the same here. Only the non-empty halves are joined, so a builder
  /// carrying just one of the two gains no stray comma. The expressions used to
  /// be dropped unless the builder came from [`SelectBuilder::raw`], which
  /// silently discarded an FTS5 `bm25(...)` projection on an ordinary table.
  pub(super) fn build_select_list(&self, params: &mut BoundParams, dialect: Dialect) -> String {
    let mut parts: Vec<String> = self.columns.clone();
    for (expr, alias) in &self.column_exprs {
      let fragment = expr.render_into(params, dialect);
      parts.push(format!(r#"{fragment} AS "{alias}""#));
    }
    parts.join(", ")
  }
}
