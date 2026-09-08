//! Core [`SelectBuilder`] struct with SQL generation methods.
//!
//! # Public API
//!
//! - [`SelectBuilder`] — fluent builder for SELECT queries
//!
//! # Usage
//!
//! ```ignore
//! let (sql, params) = SelectBuilder::new("users")
//!     .columns_raw(&["id", "name"])
//!     .to_sql();
//! ```

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::{Expr, JoinCondition, OrderBy};
use toolu_orm_core::query_column::ColumnRef;
use toolu_orm_core::value::Value;

use crate::where_clause::{append_where_for, impl_filter};

// ── JoinClause ────────────────────────────────────────────────────────────────

pub(super) struct JoinClause {
  pub join_type: &'static str,
  pub table: String,
  pub condition: JoinCondition,
}

// ── SelectBuilder ─────────────────────────────────────────────────────────────

pub struct SelectBuilder {
  pub(super) table: String,
  pub(super) columns: Vec<String>,
  pub(super) filters: Vec<Expr>,
  pub(super) joins: Vec<JoinClause>,
  pub(super) order_bys: Vec<OrderBy>,
  pub(super) limit_val: Option<i64>,
  pub(super) offset_val: Option<i64>,
  pub(super) column_exprs: Vec<(String, String)>,
  pub(super) is_raw: bool,
  pub(super) knn_applied: bool,
}

impl_filter!(SelectBuilder);

impl SelectBuilder {
  pub fn new(table: &str) -> Self {
    Self {
      table: table.to_owned(),
      columns: Vec::new(),
      filters: Vec::new(),
      joins: Vec::new(),
      order_bys: Vec::new(),
      limit_val: None,
      offset_val: None,
      column_exprs: Vec::new(),
      is_raw: false,
      knn_applied: false,
    }
  }

  pub fn raw() -> Self {
    Self {
      table: String::new(),
      columns: Vec::new(),
      filters: Vec::new(),
      joins: Vec::new(),
      order_bys: Vec::new(),
      limit_val: None,
      offset_val: None,
      column_exprs: Vec::new(),
      is_raw: true,
      knn_applied: false,
    }
  }

  pub fn columns_raw(mut self, cols: &[&str]) -> Self {
    self.columns = cols.iter().map(|c| (*c).to_owned()).collect();
    self
  }

  pub fn columns_typed(mut self, cols: &[&dyn ColumnRef]) -> Self {
    self.columns = cols.iter().map(|c| c.name().to_owned()).collect();
    self
  }

  pub fn join(mut self, table: &str, on: JoinCondition) -> Self {
    self.joins.push(JoinClause {
      join_type: "INNER JOIN",
      table: table.to_owned(),
      condition: on,
    });
    self
  }

  pub fn left_join(mut self, table: &str, on: JoinCondition) -> Self {
    self.joins.push(JoinClause {
      join_type: "LEFT JOIN",
      table: table.to_owned(),
      condition: on,
    });
    self
  }

  pub fn order_by(mut self, ob: impl Into<OrderBy>) -> Self {
    self.order_bys.push(ob.into());
    self
  }

  pub fn limit(mut self, n: i64) -> Self {
    self.limit_val = Some(n);
    self
  }

  pub fn offset(mut self, n: i64) -> Self {
    self.offset_val = Some(n);
    self
  }

  pub fn column_expr(mut self, expr: &str, alias: &str) -> Self {
    self.column_exprs.push((expr.to_owned(), alias.to_owned()));
    self
  }

  pub fn to_sql_for(&self, dialect: Dialect) -> (String, Vec<Value>) {
    let mut sql = String::new();
    let mut params: Vec<Value> = Vec::new();

    sql.push_str("SELECT ");
    sql.push_str(&self.build_select_list());

    if !self.is_raw {
      sql.push_str(&format!(r#" FROM "{}""#, self.table));
      self.append_joins(&mut sql);
    }

    append_where_for(&self.filters, &mut sql, &mut params, dialect);
    self.append_order_by(&mut sql);
    self.append_limit_offset_for(&mut sql, &mut params, dialect);

    (sql, params)
  }

  pub fn to_sql(&self) -> (String, Vec<Value>) {
    self.to_sql_for(Dialect::CURRENT)
  }

  pub fn to_count_sql_for(&self, dialect: Dialect) -> (String, Vec<Value>) {
    let mut sql = String::new();
    let mut params: Vec<Value> = Vec::new();

    sql.push_str(&format!(r#"SELECT COUNT(*) FROM "{}""#, self.table));
    self.append_joins(&mut sql);
    append_where_for(&self.filters, &mut sql, &mut params, dialect);

    (sql, params)
  }

  pub fn to_count_sql(&self) -> (String, Vec<Value>) {
    self.to_count_sql_for(Dialect::CURRENT)
  }

  pub fn to_exists_sql_for(&self, dialect: Dialect) -> (String, Vec<Value>) {
    let mut inner = String::new();
    let mut params: Vec<Value> = Vec::new();

    inner.push_str(&format!(r#"SELECT 1 FROM "{}""#, self.table));
    self.append_joins(&mut inner);
    append_where_for(&self.filters, &mut inner, &mut params, dialect);

    (format!("SELECT EXISTS({inner})"), params)
  }

  pub fn to_exists_sql(&self) -> (String, Vec<Value>) {
    self.to_exists_sql_for(Dialect::CURRENT)
  }

  pub fn table_name(&self) -> &str {
    &self.table
  }

  // ── Helpers ────────────────────────────────────────────────────────────────

  /// Plain columns first, then the aliased expressions.
  ///
  /// Only the non-empty halves are joined, so a builder carrying just one of
  /// the two gains no stray comma. The expressions used to be dropped unless
  /// the builder came from [`SelectBuilder::raw`], which silently discarded an
  /// FTS5 `bm25(...)` projection on an ordinary table.
  fn build_select_list(&self) -> String {
    let columns = self.columns.iter().map(|c| format!(r#""{c}""#));
    let exprs = self
      .column_exprs
      .iter()
      .map(|(expr, alias)| format!(r#"{expr} AS "{alias}""#));
    columns.chain(exprs).collect::<Vec<_>>().join(", ")
  }

  fn append_joins(&self, sql: &mut String) {
    for join in &self.joins {
      sql.push_str(&format!(
        r#" {} "{}" ON {}"#,
        join.join_type,
        join.table,
        join.condition.to_sql()
      ));
    }
  }

  fn append_order_by(&self, sql: &mut String) {
    if self.order_bys.is_empty() {
      return;
    }
    let parts: Vec<String> = self.order_bys.iter().map(|ob| ob.to_sql()).collect();
    sql.push_str(&format!(" ORDER BY {}", parts.join(", ")));
  }

  fn append_limit_offset_for(&self, sql: &mut String, params: &mut Vec<Value>, dialect: Dialect) {
    if let Some(limit) = self.limit_val {
      let idx = params.len() + 1;
      params.push(Value::Integer(limit));
      sql.push_str(&format!(" LIMIT {}", dialect.param(idx)));
    }
    if let Some(offset) = self.offset_val {
      let idx = params.len() + 1;
      params.push(Value::Integer(offset));
      sql.push_str(&format!(" OFFSET {}", dialect.param(idx)));
    }
  }
}
