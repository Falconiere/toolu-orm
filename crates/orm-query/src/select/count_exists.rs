//! The two aggregate forms of a SELECT: `to_count_sql_for` and
//! `to_exists_sql_for`, which ask the database *how many* and *any at all*
//! rather than for the rows themselves.

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::BoundParams;
use toolu_orm_core::value::Value;

use crate::where_clause::append_where_for;

use super::SelectBuilder;

/// The derived table a grouped or distinct count reads from.
///
/// Quoted, so it cannot collide with a keyword; distinctive, so it is unlikely
/// to collide with a real relation.
const COUNT_SUBQUERY_ALIAS: &str = r#""toolu_count""#;

impl SelectBuilder {
  /// Counts source rows after joins and filters, ignoring outer pagination.
  ///
  /// A grouped, DISTINCT or compound builder instead counts its result rows
  /// through a derived table, preserving the projection and HAVING:
  ///
  /// ```sql
  /// SELECT COUNT(*) FROM (SELECT … GROUP BY … HAVING …) AS "toolu_count"
  /// ```
  ///
  /// The plain form replaces the projection and omits HAVING. In particular,
  /// an ungrouped aggregate projection does not make this helper return `1`:
  /// it still counts the matching source rows. ORDER BY is dropped in both
  /// forms, along with LIMIT/OFFSET.
  pub fn to_count_sql_for(&self, dialect: Dialect) -> (String, Vec<Value>) {
    if self.distinct || !self.group_bys.is_empty() || self.is_compound() {
      return self.to_wrapped_count_sql_for(dialect);
    }

    let mut sql = String::new();
    let mut params = BoundParams::new();

    self.push_with_prefix(&mut sql, &mut params, dialect);
    let from_sql = self.render_from_source(&mut params, dialect);
    sql.push_str(&format!("SELECT COUNT(*) FROM {from_sql}"));
    self.append_joins(&mut sql, &mut params, dialect);
    append_where_for(&self.filters, &mut sql, &mut params, dialect);

    (sql, params.into_values())
  }

  /// Counts the rows of the unpaginated statement through a derived table.
  ///
  /// Parameters are built from scratch here, so the count's own vector is
  /// self-consistent regardless of what the paginated form would have bound.
  /// The inner statement is the whole compound, so a `UNION` is counted after
  /// its duplicates collapse. A `WITH` prefix is rendered *outside* the wrap,
  /// where SQL puts it, and therefore takes the low indices.
  fn to_wrapped_count_sql_for(&self, dialect: Dialect) -> (String, Vec<Value>) {
    let mut sql = String::new();
    let mut params = BoundParams::new();

    self.push_with_prefix(&mut sql, &mut params, dialect);
    let mut inner = String::new();
    self.push_compound(&mut inner, &mut params, dialect);
    sql.push_str(&format!(
      "SELECT COUNT(*) FROM ({inner}) AS {COUNT_SUBQUERY_ALIAS}"
    ));

    (sql, params.into_values())
  }

  /// [`Self::to_count_sql_for`] against [`Dialect::CURRENT`].
  pub fn to_count_sql(&self) -> (String, Vec<Value>) {
    self.to_count_sql_for(Dialect::CURRENT)
  }

  /// `SELECT EXISTS(SELECT 1 …)` — whether a source row or group exists.
  ///
  /// Except for compound queries, the original projection is replaced with
  /// `1`; an ungrouped aggregate projection does not affect this result.
  ///
  /// `GROUP BY`/`HAVING` are appended, because `SELECT 1 … GROUP BY x HAVING …`
  /// yields one row per surviving group and a `HAVING` can eliminate them all.
  /// `DISTINCT` is deliberately not rendered: deduplicating `SELECT 1` cannot
  /// change whether a row exists, so it would only cost the engine work.
  ///
  /// A compound is taken whole — `EXISTS(<a> UNION <b>)` — because a compound
  /// select *is* a select statement, so no derived table is needed. A `WITH`
  /// prefix stays outside the `EXISTS`, where SQL puts it.
  pub fn to_exists_sql_for(&self, dialect: Dialect) -> (String, Vec<Value>) {
    let mut sql = String::new();
    let mut params = BoundParams::new();
    let mut inner = String::new();

    self.push_with_prefix(&mut sql, &mut params, dialect);
    if self.is_compound() {
      self.push_compound(&mut inner, &mut params, dialect);
    } else {
      let from_sql = self.render_from_source(&mut params, dialect);
      inner.push_str(&format!("SELECT 1 FROM {from_sql}"));
      self.append_joins(&mut inner, &mut params, dialect);
      append_where_for(&self.filters, &mut inner, &mut params, dialect);
      self.append_group_by(&mut inner, &mut params, dialect);
      self.append_having(&mut inner, &mut params, dialect);
    }
    sql.push_str(&format!("SELECT EXISTS({inner})"));

    (sql, params.into_values())
  }

  /// [`Self::to_exists_sql_for`] against [`Dialect::CURRENT`].
  pub fn to_exists_sql(&self) -> (String, Vec<Value>) {
    self.to_exists_sql_for(Dialect::CURRENT)
  }
}
