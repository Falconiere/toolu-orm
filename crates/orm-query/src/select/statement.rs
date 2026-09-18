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
  /// The row bound of [`SelectBuilder::to_first_row_sql_for`] is the only
  /// caller that passes something other than `self.limit_val`.
  pub(super) fn to_sql_with_limit(
    &self,
    dialect: Dialect,
    limit: Option<i64>,
  ) -> (String, Vec<Value>) {
    let mut sql = String::new();
    let mut params: Vec<Value> = Vec::new();

    self.push_statement(&mut sql, &mut params, dialect, limit);

    (sql, params)
  }

  /// The whole statement appended to a caller's `sql` and `params`.
  ///
  /// Everything that nests a statement — a CTE body, a set-operation arm, a
  /// subquery — goes through here with the *same* vectors, which is what makes
  /// bind numbering across a statement boundary free: every clause derives its
  /// own first index from the live `params.len()`, so a nested statement
  /// continues the count instead of restarting it.
  ///
  /// Clauses render in the order SQL requires: the `WITH` prefix, the compound
  /// (`<core> [UNION [ALL] <core>]*`), then `ORDER BY`, then `LIMIT`/`OFFSET`.
  /// The prefix and the tail belong to the whole statement rather than to any
  /// one arm, which is why they sit outside the core.
  pub(super) fn push_statement(
    &self,
    sql: &mut String,
    params: &mut Vec<Value>,
    dialect: Dialect,
    limit: Option<i64>,
  ) {
    self.push_with_prefix(sql, params, dialect);
    self.push_compound(sql, params, dialect);
    self.append_order_by(sql, params, dialect);
    self.append_limit_offset_for(sql, params, dialect, limit);
  }

  /// `SELECT … FROM … JOIN … WHERE … GROUP BY … HAVING …` — the clause run
  /// with no tail.
  ///
  /// One definition, shared by the plain statement and by the derived table a
  /// grouped or distinct count wraps, so the two cannot disagree about what
  /// they count.
  pub(super) fn push_core(&self, sql: &mut String, params: &mut Vec<Value>, dialect: Dialect) {
    self.push_select_head(sql, params, dialect);
    self.append_joins(sql, params, dialect);
    append_where_for(&self.filters, sql, params, dialect);
    self.append_group_by(sql, params, dialect);
    self.append_having(sql, params, dialect);
  }

  /// `SELECT [DISTINCT] <select list>[ FROM <source>]`.
  ///
  /// The source renders through `TableRef::to_sql_fragment_for`, because a
  /// table-valued function binds its arguments there — after the select list's
  /// own placeholders, which is where it is written. A
  /// [`SelectBuilder::raw`] builder has no source, so it renders no `FROM`.
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
      let from_sql = self.render_from_source(params, dialect);
      sql.push_str(&format!(" FROM {from_sql}"));
    }
  }

  /// The `FROM` source, pushing whatever its arguments bind.
  pub(super) fn render_from_source(&self, params: &mut Vec<Value>, dialect: Dialect) -> String {
    let start = params.len() + 1;
    let (fragment, source_params) = self.table.to_sql_fragment_for(start, dialect);
    params.extend(source_params);
    fragment
  }

  /// [`Self::to_sql_for`] against [`Dialect::CURRENT`].
  pub fn to_sql(&self) -> (String, Vec<Value>) {
    self.to_sql_for(Dialect::CURRENT)
  }
}
