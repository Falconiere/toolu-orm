//! The scalar node: [`Scalar`], its kinds, and its leaf constructors.

use crate::dialect::Dialect;
use crate::expr::render::render_scalar;
use crate::expr::Expr;
use crate::query_column::Column;
use crate::value::Value;

/// A value-producing SQL expression.
///
/// Build one from a column, a bound value, a function call, arithmetic or a
/// `CASE`, then compose it into a predicate ([`Scalar::gte`] and friends), a
/// projection, an `ORDER BY` term, an inserted value or an updated column.
/// Every bound value it carries is rendered as a dialect placeholder, never
/// interpolated.
///
/// ```
/// use toolu_orm_core::column::Integer;
/// use toolu_orm_core::dialect::Dialect;
/// use toolu_orm_core::expr::Scalar;
/// use toolu_orm_core::query_column::Column;
///
/// const HITS: Column<Integer> = Column::new("pages", "hits");
///
/// let bumped = Scalar::col(&HITS) + Scalar::bind(1);
/// let (sql, params) = bumped.to_sql_fragment_for(1, Dialect::Sqlite);
/// assert_eq!(sql, r#"("pages"."hits" + ?1)"#);
/// assert_eq!(params, vec![toolu_orm_core::value::Value::Integer(1)]);
/// ```
pub struct Scalar {
  pub(crate) kind: ScalarKind,
}

/// Scalar node kinds; rendered by `render_scalar` per dialect.
///
/// Private, like [`crate::expr::Expr`]'s own kinds, so later query features
/// can add nodes without breaking the published surface.
pub(crate) enum ScalarKind {
  /// Pre-rendered SQL text plus the values its `?` placeholders bind.
  Raw { sql: String, params: Vec<Value> },
  /// One bound value, rendered as `?N` / `$N`.
  Bind(Value),
  /// `name(arg, ...)` with a validated function name.
  Func { name: String, args: Vec<Scalar> },
  /// `(left <op> right)` for arithmetic and `||` concatenation.
  Arith {
    left: Box<Scalar>,
    op: &'static str,
    right: Box<Scalar>,
  },
  /// `CASE WHEN <predicate> THEN <value> ... [ELSE <value>] END`, always with
  /// at least one branch.
  Case {
    branches: Vec<(Expr, Scalar)>,
    otherwise: Option<Box<Scalar>>,
  },
}

impl Scalar {
  /// The column itself, qualified as `"table"."name"`.
  #[must_use]
  pub fn col<T>(column: &Column<T>) -> Self {
    Self::sql(column.qualified())
  }

  /// One bound value.
  #[must_use]
  pub fn bind(value: impl Into<Value>) -> Self {
    Self {
      kind: ScalarKind::Bind(value.into()),
    }
  }

  /// SQL text that binds nothing — an identifier, a literal, or a call this
  /// crate already rendered, such as `bm25("posts")` or a `vec0` distance.
  ///
  /// The text still goes through placeholder renumbering, so a bare `?` in it
  /// would take an index without supplying a value: pass such a fragment to
  /// [`Scalar::raw`] with its values instead.
  #[must_use]
  pub fn sql(sql: impl Into<String>) -> Self {
    Self::raw(sql, Vec::new())
  }

  /// SQL text whose bare `?` placeholders bind `params`, renumbered for the
  /// position this node ends up in — the scalar twin of [`crate::expr::Expr::raw`].
  #[must_use]
  pub fn raw(sql: impl Into<String>, params: Vec<Value>) -> Self {
    Self {
      kind: ScalarKind::Raw {
        sql: sql.into(),
        params,
      },
    }
  }

  pub(crate) fn from_kind(kind: ScalarKind) -> Self {
    Self { kind }
  }

  /// Render with dialect-specific placeholders starting at `start`, returning
  /// the SQL and the values it binds, in emission order.
  #[must_use]
  pub fn to_sql_fragment_for(&self, start: usize, dialect: Dialect) -> (String, Vec<Value>) {
    let mut params: Vec<Value> = Vec::new();
    let sql = render_scalar(&self.kind, start, &mut params, dialect);
    (sql, params)
  }

  /// [`Scalar::to_sql_fragment_for`] against [`Dialect::CURRENT`].
  #[must_use]
  pub fn to_sql_fragment(&self, start: usize) -> (String, Vec<Value>) {
    self.to_sql_fragment_for(start, Dialect::CURRENT)
  }
}
