//! One `JOIN` of a [`SelectBuilder`], and how the list of them renders.

use toolu_orm_core::alias::TableRef;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::{BoundParams, JoinCondition};

use super::SelectBuilder;

/// A joined table and the `ON` predicate that attaches it.
pub(super) struct JoinClause {
  join_type: &'static str,
  table: TableRef,
  condition: JoinCondition,
}

impl JoinClause {
  /// `INNER JOIN <table> ON <condition>`.
  pub(super) fn inner(table: TableRef, condition: JoinCondition) -> Self {
    Self {
      join_type: "INNER JOIN",
      table,
      condition,
    }
  }

  /// `LEFT JOIN <table> ON <condition>`.
  pub(super) fn left(table: TableRef, condition: JoinCondition) -> Self {
    Self {
      join_type: "LEFT JOIN",
      table,
      condition,
    }
  }
}

impl SelectBuilder {
  /// Appends every join, binding whatever its source and its `ON` clause
  /// carry.
  ///
  /// Both halves number from the live `params.len() + 1` and push as they are
  /// written, so the indices follow render order: the joined source first —
  /// a table-valued function binds its arguments there — then its `ON`. The
  /// clauses around them do the same, so the whole statement's parameter
  /// vector is in bind order by construction.
  pub(super) fn append_joins(&self, sql: &mut String, params: &mut BoundParams, dialect: Dialect) {
    for join in &self.joins {
      let table_start = params.next_index();
      let (table_sql, table_params) = join.table.to_sql_fragment_for(table_start, dialect);
      params.extend(table_params);
      let on_sql = join.condition.render_into(params, dialect);
      sql.push_str(&format!(" {} {table_sql} ON {on_sql}", join.join_type));
    }
  }
}
