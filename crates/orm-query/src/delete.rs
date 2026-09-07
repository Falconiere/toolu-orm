//! DELETE query builder with dialect-aware SQL generation.

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::Expr;
use toolu_orm_core::value::Value;

use crate::where_clause::{append_where_for, cfg_single_backend, impl_filter};

cfg_single_backend! {
  use crate::exec_helpers::impl_execute;
}

// ── DeleteBuilder ─────────────────────────────────────────────────────────────

pub struct DeleteBuilder {
  table: String,
  filters: Vec<Expr>,
}

impl_filter!(DeleteBuilder);

impl DeleteBuilder {
  pub fn new(table: &str) -> Self {
    Self {
      table: table.to_owned(),
      filters: Vec::new(),
    }
  }

  pub fn to_sql_for(&self, dialect: Dialect) -> (String, Vec<Value>) {
    let mut sql = String::new();
    let mut params: Vec<Value> = Vec::new();

    sql.push_str(&format!(r#"DELETE FROM "{}""#, self.table));
    append_where_for(&self.filters, &mut sql, &mut params, dialect);

    (sql, params)
  }

  pub fn to_sql(&self) -> (String, Vec<Value>) {
    self.to_sql_for(Dialect::CURRENT)
  }
}

cfg_single_backend! {
  impl_execute!(DeleteBuilder, "DELETE");
}
