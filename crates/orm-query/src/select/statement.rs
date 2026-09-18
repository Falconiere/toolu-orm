//! The full `SELECT` statement: how [`SelectBuilder`] renders its clauses and
//! numbers their parameters.

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::value::Value;

use crate::where_clause::append_where_for;

use super::SelectBuilder;

impl SelectBuilder {
  /// The whole statement for `dialect`, with its parameters in bind order.
  pub fn to_sql_for(&self, dialect: Dialect) -> (String, Vec<Value>) {
    self.to_sql_with_limit(dialect, self.limit_val)
  }

  /// [`Self::to_sql_for`] with `limit` standing in for [`SelectBuilder::limit`].
  ///
  /// Everything else — select list, joins, `WHERE`, `GROUP BY`, `HAVING`,
  /// `ORDER BY`, `OFFSET` — is rendered exactly as `to_sql_for` renders it. The row bound of
  /// [`SelectBuilder::to_first_row_sql_for`] is the only caller that passes
  /// something other than `self.limit_val`.
  ///
  /// Clauses render in the order SQL requires, and every one of them derives
  /// its own first placeholder index from `params.len() + 1` and pushes values
  /// as it writes them. So the returned vector is in bind order by
  /// construction: select list → `ON` → `WHERE` → `GROUP BY` → `HAVING` →
  /// `ORDER BY` → `LIMIT`/`OFFSET`.
  pub(super) fn to_sql_with_limit(
    &self,
    dialect: Dialect,
    limit: Option<i64>,
  ) -> (String, Vec<Value>) {
    let mut sql = String::new();
    let mut params: Vec<Value> = Vec::new();

    self.push_select_head(&mut sql, &mut params, dialect);
    self.append_joins(&mut sql, &mut params, dialect);
    append_where_for(&self.filters, &mut sql, &mut params, dialect);
    self.append_group_by(&mut sql, &mut params, dialect);
    self.append_having(&mut sql, &mut params, dialect);
    self.append_order_by(&mut sql, &mut params, dialect);
    self.append_limit_offset_for(&mut sql, &mut params, dialect, limit);

    (sql, params)
  }

  /// `SELECT [DISTINCT] <select list>[ FROM <table>]`.
  ///
  /// Shared with the counted form, which wraps this same head in a derived
  /// table, so the two cannot disagree about what `DISTINCT` deduplicates on.
  /// A [`SelectBuilder::raw`] builder has no table, so it renders no `FROM`.
  pub(super) fn push_select_head(
    &self,
    sql: &mut String,
    params: &mut Vec<Value>,
    dialect: Dialect,
  ) {
    sql.push_str("SELECT ");
    if self.distinct {
      sql.push_str("DISTINCT ");
    }
    let select_list = self.build_select_list(params, dialect);
    sql.push_str(&select_list);

    if !self.is_raw {
      sql.push_str(&format!(" FROM {}", self.table.to_sql()));
    }
  }

  /// [`Self::to_sql_for`] against [`Dialect::CURRENT`].
  pub fn to_sql(&self) -> (String, Vec<Value>) {
    self.to_sql_for(Dialect::CURRENT)
  }
}
