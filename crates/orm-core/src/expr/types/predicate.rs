//! The boolean node: [`Expr`], its kinds, and the constructors that build them.

use crate::dialect::Dialect;
use crate::expr::binding::{BindSource, ListSource};
use crate::expr::render::render_expr;
use crate::expr::{BoundParams, Scalar, SelectSource};
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
    value: BindSource,
  },
  InList {
    column: String,
    values: ListSource,
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
  /// `[NOT ]EXISTS (<statement>)`.
  Exists {
    query: Box<dyn SelectSource>,
    negated: bool,
  },
  /// `<left> [NOT ]IN (<statement>)`.
  InSubquery {
    left: Scalar,
    query: Box<dyn SelectSource>,
    negated: bool,
  },
  And(Box<Expr>, Box<Expr>),
  Or(Box<Expr>, Box<Expr>),
  Raw {
    sql: String,
    params: Vec<Value>,
  },
}

impl Expr {
  /// `value` is either the node's own [`Value`] or a [`SharedBind`] handle —
  /// both convert, so every existing call site is unchanged.
  ///
  /// [`SharedBind`]: crate::expr::SharedBind
  pub(crate) fn comparison(column: String, op: &'static str, value: impl Into<BindSource>) -> Self {
    Self {
      kind: ExprKind::Comparison {
        column,
        op,
        value: value.into(),
      },
    }
  }

  /// `values` is either the node's own `Vec<Value>` or a [`SharedBindList`].
  ///
  /// [`SharedBindList`]: crate::expr::SharedBindList
  pub(crate) fn in_list(column: String, values: impl Into<ListSource>, negated: bool) -> Self {
    Self {
      kind: ExprKind::InList {
        column,
        values: values.into(),
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

  /// An `Expr` around a node built elsewhere in the crate, the twin of
  /// [`Scalar::from_kind`](crate::expr::Scalar).
  pub(crate) fn from_kind(kind: ExprKind) -> Self {
    Self { kind }
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

  /// Renders into `params`, appending what it binds and sharing the buffer's
  /// binding ledger — so a [`SharedBind`] used here and elsewhere in the same
  /// statement takes one placeholder.
  ///
  /// The fragment numbers from [`BoundParams::next_index`]; a caller passes no
  /// offset, because position lives in the buffer.
  ///
  /// [`SharedBind`]: crate::expr::SharedBind
  #[must_use]
  pub fn render_into(&self, params: &mut BoundParams, dialect: Dialect) -> String {
    render_expr(&self.kind, params, dialect)
  }

  /// Generate SQL fragment with dialect-specific positional parameters
  /// starting at `start`. Returns `(sql_string, params_vec)`.
  ///
  /// [`Self::render_into`] into a fresh buffer positioned at `start`, so a
  /// handle used twice *within* this fragment still shares one placeholder,
  /// while a handle it shares with another fragment binds in each — a
  /// standalone fragment owns its own numbering, as it always has.
  pub fn to_sql_fragment_for(&self, start: usize, dialect: Dialect) -> (String, Vec<Value>) {
    let mut params = BoundParams::new();
    let sql = params.nested(start, |nested| self.render_into(nested, dialect));
    (sql, params.into_values())
  }

  /// Generate SQL fragment with positional parameters starting at `start`.
  /// Uses the compile-time selected dialect.
  /// Returns `(sql_string, params_vec)`.
  pub fn to_sql_fragment(&self, start: usize) -> (String, Vec<Value>) {
    self.to_sql_fragment_for(start, Dialect::CURRENT)
  }
}
