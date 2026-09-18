//! The `ORDER BY` tail of a SELECT.

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::OrderBy;
use toolu_orm_core::value::Value;

use super::SelectBuilder;

impl SelectBuilder {
  /// One more `ORDER BY` term, applied after the ones already added.
  ///
  /// Takes anything that converts into an [`OrderBy`]: `Column::asc`,
  /// `Scalar::asc` for a computed term, `OrderBy::alias_asc` for a
  /// projection's alias, or a distance type's own `asc`.
  pub fn order_by(mut self, ob: impl Into<OrderBy>) -> Self {
    self.order_bys.push(ob.into());
    self
  }

  /// A term may bind (a `CASE`, or a call over a bound value), so each one is
  /// rendered from the count of parameters already emitted — after the
  /// `WHERE` clause's and before `LIMIT`/`OFFSET`'s.
  pub(super) fn append_order_by(
    &self,
    sql: &mut String,
    params: &mut Vec<Value>,
    dialect: Dialect,
  ) {
    if self.order_bys.is_empty() {
      return;
    }
    let mut parts: Vec<String> = Vec::with_capacity(self.order_bys.len());
    for ob in &self.order_bys {
      let start = params.len() + 1;
      let (fragment, ob_params) = ob.to_sql_fragment_for(start, dialect);
      params.extend(ob_params);
      parts.push(fragment);
    }
    sql.push_str(&format!(" ORDER BY {}", parts.join(", ")));
  }
}
