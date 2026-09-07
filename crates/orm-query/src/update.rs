//! UPDATE query builder with SET clause and filter support.

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::Expr;
use toolu_orm_core::query_column::Column;
use toolu_orm_core::value::Value;

use crate::where_clause::{append_where_for, cfg_single_backend, impl_filter};

cfg_single_backend! {
  use crate::exec_helpers::impl_execute;
}

// ── SetClause ─────────────────────────────────────────────────────────────────

enum SetClause {
  Value { column: String, value: Value },
  Expr { column: String, sql: String },
}

// ── UpdateBuilder ─────────────────────────────────────────────────────────────

pub struct UpdateBuilder {
  table: String,
  sets: Vec<SetClause>,
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

  pub fn set<T>(mut self, col: &Column<T>, val: impl Into<Value>) -> Self {
    self.sets.push(SetClause::Value {
      column: col.name.to_owned(),
      value: val.into(),
    });
    self
  }

  pub fn set_expr<T>(mut self, col: &Column<T>, expr: &str) -> Self {
    self.sets.push(SetClause::Expr {
      column: col.name.to_owned(),
      sql: expr.to_owned(),
    });
    self
  }

  pub fn to_sql_for(&self, dialect: Dialect) -> (String, Vec<Value>) {
    let mut sql = String::new();
    let mut params: Vec<Value> = Vec::new();

    let mut set_params: Vec<Value> = Vec::new();
    for clause in &self.sets {
      if let SetClause::Value { value, .. } = clause {
        set_params.push(value.clone());
      }
    }

    sql.push_str(&format!(r#"UPDATE "{}""#, self.table));
    sql.push_str(" SET ");

    let mut set_idx = 1usize;
    let set_parts: Vec<String> = self
      .sets
      .iter()
      .map(|clause| match clause {
        SetClause::Value { column, .. } => {
          let part = format!(r#""{column}" = {}"#, dialect.param(set_idx));
          set_idx += 1;
          part
        },
        SetClause::Expr {
          column,
          sql: expr_sql,
        } => {
          format!(r#""{column}" = {expr_sql}"#)
        },
      })
      .collect();

    sql.push_str(&set_parts.join(", "));
    params.extend(set_params);

    append_where_for(&self.filters, &mut sql, &mut params, dialect);

    (sql, params)
  }

  pub fn to_sql(&self) -> (String, Vec<Value>) {
    self.to_sql_for(Dialect::CURRENT)
  }
}

cfg_single_backend! {
  impl_execute!(UpdateBuilder, "UPDATE");
}
