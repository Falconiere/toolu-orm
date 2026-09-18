//! `DISTINCT`, `GROUP BY` and `HAVING`: the clauses that turn a row listing
//! into a deduplicated one or a grouped report.

use toolu_orm_core::alias::QualifiedColumn;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::{BoundParams, Expr, Scalar};

use crate::where_clause::append_conjuncts_for;

use super::SelectBuilder;

impl SelectBuilder {
  /// `SELECT DISTINCT …` — collapses rows that agree on every projected
  /// column.
  ///
  /// The engine deduplicates before `ORDER BY`, `LIMIT` and `OFFSET`, so a
  /// page of a distinct listing is a page of *distinct* rows, not a
  /// deduplicated page of raw ones. Calling it twice changes nothing.
  ///
  /// Postgres requires every `ORDER BY` term of a `DISTINCT` query to appear
  /// in the select list, so order by a projected term — `columns_qualified`
  /// pairs with a typed column's `asc()`/`desc()`, which both render
  /// `"table"."column"`.
  pub fn distinct(mut self) -> Self {
    self.distinct = true;
    self
  }

  /// One more `GROUP BY` term from a typed column.
  ///
  /// Takes `Column<T>` and `AliasedColumn<T>` alike and always renders
  /// qualified, so a grouping key stays unambiguous after a join.
  pub fn group_by(self, column: &dyn QualifiedColumn) -> Self {
    self.group_by_scalar(Scalar::sql(column.qualified()))
  }

  /// One more `GROUP BY` term from any [`Scalar`] — a computed key, or
  /// `Scalar::sql(...)` as the raw escape hatch.
  ///
  /// Terms render in call order, alongside those added by
  /// [`Self::group_by`].
  pub fn group_by_scalar(mut self, term: Scalar) -> Self {
    self.group_bys.push(term);
    self
  }

  /// One more `HAVING` conjunct, `AND`-joined with the others.
  ///
  /// `HAVING` filters groups where `filter` filters rows, so its predicate is
  /// normally built from an aggregate:
  /// `having(Scalar::count_star().gt(Scalar::bind(1)))`. Repeat the aggregate
  /// rather than naming a projection's alias — SQLite resolves the alias,
  /// Postgres does not.
  pub fn having(mut self, expr: Expr) -> Self {
    self.havings.push(expr);
    self
  }

  /// Appends `GROUP BY <term>, …`, binding whatever the terms carry.
  ///
  /// Rendered after `WHERE` and before `HAVING`, each term numbering from
  /// `params.len() + 1` like every other clause.
  pub(super) fn append_group_by(
    &self,
    sql: &mut String,
    params: &mut BoundParams,
    dialect: Dialect,
  ) {
    if self.group_bys.is_empty() {
      return;
    }
    let mut parts: Vec<String> = Vec::with_capacity(self.group_bys.len());
    for term in &self.group_bys {
      parts.push(term.render_into(params, dialect));
    }
    sql.push_str(&format!(" GROUP BY {}", parts.join(", ")));
  }

  /// Appends `HAVING <conjunct> AND …` through the same renderer `WHERE` uses,
  /// so the two clauses cannot drift.
  pub(super) fn append_having(&self, sql: &mut String, params: &mut BoundParams, dialect: Dialect) {
    append_conjuncts_for(&self.havings, " HAVING ", sql, params, dialect);
  }
}
