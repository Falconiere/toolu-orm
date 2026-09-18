//! One `JOIN` of a [`SelectBuilder`], and how the list of them renders.

use toolu_orm_core::alias::TableRef;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::JoinCondition;
use toolu_orm_core::value::Value;

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
  /// Appends every join, binding whatever its `ON` clause carries.
  ///
  /// Each condition numbers its placeholders from `params.len() + 1` and its
  /// values are pushed as they are written, so the indices follow render
  /// order: in `to_sql_with_limit` the select list comes first — it can bind
  /// through `column_scalar` — then `ON`, then `WHERE`, then `ORDER BY` and
  /// `LIMIT`/`OFFSET`. In `to_count_sql_for` and `to_exists_sql_for` no select
  /// list is rendered, so `ON` starts at 1. `append_where_for` derives its own
  /// start the same way and continues after whatever came before it.
  pub(super) fn append_joins(&self, sql: &mut String, params: &mut Vec<Value>, dialect: Dialect) {
    for join in &self.joins {
      let start = params.len() + 1;
      let (on_sql, on_params) = join.condition.to_sql_fragment_for(start, dialect);
      params.extend(on_params);
      sql.push_str(&format!(
        " {} {} ON {on_sql}",
        join.join_type,
        join.table.to_sql()
      ));
    }
  }
}
