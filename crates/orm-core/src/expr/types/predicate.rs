//! The boolean node: [`Expr`], its kinds, and the constructors that build them.

use crate::dialect::Dialect;
use crate::expr::render::render_expr;
use crate::expr::Scalar;
use crate::value::Value;

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
  Match {
    target: String,
    pattern: Value,
  },
  /// Postgres `document @@ query_fn(config, $N)`.
  TsMatch {
    document: String,
    query_fn: &'static str,
    config: String,
    pattern: Value,
  },
  /// `<left> <op> <right>` between two scalars.
  Compare {
    left: Scalar,
    op: &'static str,
    right: Scalar,
  },
  /// `<left> LIKE <pattern> [ESCAPE ?N]`.
  Like {
    left: Scalar,
    pattern: Scalar,
    escape: Option<char>,
  },
  And(Box<Expr>, Box<Expr>),
  Or(Box<Expr>, Box<Expr>),
  Raw {
    sql: String,
    params: Vec<Value>,
  },
}

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

  pub(crate) fn compare(left: Scalar, op: &'static str, right: Scalar) -> Self {
    Self {
      kind: ExprKind::Compare { left, op, right },
    }
  }

  pub(crate) fn like_node(left: Scalar, pattern: Scalar, escape: Option<char>) -> Self {
    Self {
      kind: ExprKind::Like {
        left,
        pattern,
        escape,
      },
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
