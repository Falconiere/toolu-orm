//! Expression AST for WHERE clause generation across dialects.

use crate::dialect::Dialect;
use crate::query_column::Column;
use crate::value::Value;

// ── Public types ──────────────────────────────────────────────────────────────

pub struct Expr {
  pub(crate) kind: ExprKind,
}

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

pub struct OrderBy {
  pub(crate) column: String,
  pub(crate) direction: &'static str,
}

pub struct JoinCondition {
  pub(crate) left: String,
  pub(crate) right: String,
}

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

// ── SQL rendering ─────────────────────────────────────────────────────────────

fn render_expr(kind: &ExprKind, start: usize, params: &mut Vec<Value>, dialect: Dialect) -> String {
  match kind {
    ExprKind::Comparison { column, op, value } => {
      let idx = start + params.len();
      params.push(value.clone());
      format!("{column} {op} {}", dialect.param(idx))
    },
    ExprKind::InList {
      column,
      values,
      negated,
    } => {
      let base = start + params.len();
      let placeholders: Vec<String> = values
        .iter()
        .enumerate()
        .map(|(i, _)| dialect.param(base + i))
        .collect();
      params.extend(values.iter().cloned());
      let keyword = if *negated { "NOT IN" } else { "IN" };
      format!("{column} {keyword} ({})", placeholders.join(", "))
    },
    ExprKind::IsNull { column, negated } => {
      if *negated {
        format!("{column} IS NOT NULL")
      } else {
        format!("{column} IS NULL")
      }
    },
    ExprKind::Between { column, low, high } => {
      let low_idx = start + params.len();
      params.push(low.clone());
      let high_idx = start + params.len();
      params.push(high.clone());
      format!(
        "{column} BETWEEN {} AND {}",
        dialect.param(low_idx),
        dialect.param(high_idx)
      )
    },
    ExprKind::And(left, right) => {
      let left_sql = render_expr(&left.kind, start, params, dialect);
      let right_sql = render_expr(&right.kind, start, params, dialect);
      format!("({left_sql} AND {right_sql})")
    },
    ExprKind::Or(left, right) => {
      let left_sql = render_expr(&left.kind, start, params, dialect);
      let right_sql = render_expr(&right.kind, start, params, dialect);
      format!("({left_sql} OR {right_sql})")
    },
    ExprKind::Raw {
      sql,
      params: raw_params,
    } => {
      params.extend(raw_params.iter().cloned());
      number_raw_params(sql, start, dialect)
    },
  }
}

/// Replace bare `?` (not already `?N`) with sequential placeholders starting at `start`.
fn number_raw_params(sql: &str, start: usize, dialect: Dialect) -> String {
  let mut result = String::with_capacity(sql.len() + 8);
  let mut counter = start;
  let mut chars = sql.chars().peekable();

  while let Some(ch) = chars.next() {
    if ch != '?' {
      result.push(ch);
      continue;
    }
    // ch == '?': check if next char is a digit (already numbered)
    if chars.peek().is_some_and(|c| c.is_ascii_digit()) {
      let mut num_str = String::new();
      while chars.peek().is_some_and(|c| c.is_ascii_digit()) {
        if let Some(d) = chars.next() {
          num_str.push(d);
        }
      }
      let idx: usize = num_str.parse().unwrap_or(counter);
      result.push_str(&dialect.param(idx));
    } else {
      result.push_str(&dialect.param(counter));
      counter += 1;
    }
  }

  result
}
