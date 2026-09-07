//! Expression types (`Expr`, `OrderBy`, `JoinCondition`, `JsonExpr`) and their
//! constructors; rendering lives in `render.rs`.

use crate::dialect::Dialect;
use crate::query_column::Column;
use crate::value::Value;

use super::render::render_expr;

// ── Public types ──────────────────────────────────────────────────────────────

/// A WHERE-clause expression tree; render with [`Expr::to_sql_fragment_for`].
pub struct Expr {
  pub(crate) kind: ExprKind,
}

/// Expression node kinds; rendered by `render_expr` per dialect.
pub(crate) enum ExprKind {
  Comparison {
    column: String,
    op: &'static str,
    value: Value,
  },
  InList {
    column: String,
    values: Vec<Value>,
    negated: bool,
  },
  IsNull {
    column: String,
    negated: bool,
  },
  Between {
    column: String,
    low: Value,
    high: Value,
  },
  And(Box<Expr>, Box<Expr>),
  Or(Box<Expr>, Box<Expr>),
  Raw {
    sql: String,
    params: Vec<Value>,
  },
}

/// One `ORDER BY` term, built via `Column::asc` / `Column::desc`.
pub struct OrderBy {
  pub(crate) column: String,
  pub(crate) direction: &'static str,
}

/// `left = right` pair for `JOIN ... ON`.
pub struct JoinCondition {
  pub(crate) left: String,
  pub(crate) right: String,
}

/// A JSON access fragment usable inside expressions.
pub struct JsonExpr {
  fragment: String,
}

// ── Expr constructors (pub(crate)) ────────────────────────────────────────────

impl Expr {
  pub(crate) fn comparison(column: String, op: &'static str, value: Value) -> Self {
    Self {
      kind: ExprKind::Comparison { column, op, value },
    }
  }

  pub(crate) fn in_list(column: String, values: Vec<Value>, negated: bool) -> Self {
    Self {
      kind: ExprKind::InList {
        column,
        values,
        negated,
      },
    }
  }

  pub(crate) fn is_null(column: String, negated: bool) -> Self {
    Self {
      kind: ExprKind::IsNull { column, negated },
    }
  }

  pub(crate) fn between(column: String, low: Value, high: Value) -> Self {
    Self {
      kind: ExprKind::Between { column, low, high },
    }
  }

  // ── Public API ────────────────────────────────────────────────────────────

  pub fn raw(sql: impl Into<String>, params: Vec<Value>) -> Self {
    Self {
      kind: ExprKind::Raw {
        sql: sql.into(),
        params,
      },
    }
  }

  pub fn json_extract<T>(col: &Column<T>, path: &str) -> JsonExpr {
    let fragment = format!(r#"json_extract({}, '{}')"#, col.qualified(), path);
    JsonExpr { fragment }
  }

  pub fn and(self, other: Expr) -> Expr {
    Expr {
      kind: ExprKind::And(Box::new(self), Box::new(other)),
    }
  }

  pub fn or(self, other: Expr) -> Expr {
    Expr {
      kind: ExprKind::Or(Box::new(self), Box::new(other)),
    }
  }

  /// Generate SQL fragment with dialect-specific positional parameters
  /// starting at `start`. Returns `(sql_string, params_vec)`.
  pub fn to_sql_fragment_for(&self, start: usize, dialect: Dialect) -> (String, Vec<Value>) {
    let mut params: Vec<Value> = Vec::new();
    let sql = render_expr(&self.kind, start, &mut params, dialect);
    (sql, params)
  }

  /// Generate SQL fragment with positional parameters starting at `start`.
  /// Uses the compile-time selected dialect.
  /// Returns `(sql_string, params_vec)`.
  pub fn to_sql_fragment(&self, start: usize) -> (String, Vec<Value>) {
    self.to_sql_fragment_for(start, Dialect::CURRENT)
  }
}

// ── OrderBy ───────────────────────────────────────────────────────────────────

impl OrderBy {
  pub fn to_sql(&self) -> String {
    format!("{} {}", self.column, self.direction)
  }
}

// ── JoinCondition ──────────────────────────────────────────────────────────────

impl JoinCondition {
  pub fn to_sql(&self) -> String {
    format!("{} = {}", self.left, self.right)
  }
}

// ── JsonExpr ──────────────────────────────────────────────────────────────────

impl JsonExpr {
  pub fn eq<V: Into<Value>>(self, val: V) -> Expr {
    Expr::comparison(self.fragment, "=", val.into())
  }

  pub fn like<V: Into<Value>>(self, val: V) -> Expr {
    Expr::comparison(self.fragment, "LIKE", val.into())
  }
}
