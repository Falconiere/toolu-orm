//! The scalar node: [`Scalar`], its kinds, and its leaf constructors.

use crate::dialect::Dialect;
use crate::expr::binding::BindSource;
use crate::expr::render::render_scalar;
use crate::expr::{BoundParams, Expr, SelectSource, SharedBind};
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
  /// One bound value, rendered as `?N` / `$N` — its own, or a shared handle's.
  Bind(BindSource),
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
  /// `FUNC(*)`, `FUNC(<arg>)` or `FUNC(DISTINCT <arg>)` — an aggregate whose
  /// name this crate chooses, so it needs no validation.
  Aggregate {
    func: &'static str,
    arg: AggregateArg,
  },
  /// `(SELECT …)` — a whole statement read as one value.
  Subquery(Box<dyn SelectSource>),
}

/// What sits between an aggregate's parentheses.
///
/// An enum rather than a `distinct: bool` beside an `Option<Scalar>`, because
/// that pair can spell `COUNT(DISTINCT *)`, which no engine accepts. Every
/// constructor in `super::aggregate` produces a valid combination, so the
/// invalid one is unrepresentable.
pub(crate) enum AggregateArg {
  /// `*` — count rows rather than values.
  Star,
  /// A plain argument: `SUM("t"."n")`.
  All(Box<Scalar>),
  /// `DISTINCT <arg>` — aggregate each distinct value once.
  Distinct(Box<Scalar>),
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
      kind: ScalarKind::Bind(BindSource::Owned(value.into())),
    }
  }

  /// A [`SharedBind`] in value position — the reusable twin of
  /// [`Scalar::bind`].
  ///
  /// Every scalar built from one handle renders the same placeholder, so the
  /// value is bound once however many projections, comparisons, `CASE` arms or
  /// assignments name it.
  #[must_use]
  pub fn shared(bind: &SharedBind) -> Self {
    Self {
      kind: ScalarKind::Bind(BindSource::Shared(bind.clone())),
    }
  }

  /// SQL text that binds nothing — an identifier, a literal, or a call this
  /// crate already rendered, such as `bm25("posts")` or a `vec0` distance.
  ///
  /// The text still goes through placeholder renumbering, so a `?` or a `?N`
  /// in it would take an index without supplying a value: pass such a fragment
  /// to [`Scalar::raw`] with its values instead.
  #[must_use]
  pub fn sql(sql: impl Into<String>) -> Self {
    Self::raw(sql, Vec::new())
  }

  /// SQL text whose placeholders bind `params`, renumbered for the position
  /// this node ends up in — the scalar twin of [`crate::expr::Expr::raw`],
  /// with the same rule: a bare `?` takes the next free index and `?N`
  /// addresses this fragment's own `N`-th value.
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

  /// Renders into `params`, sharing its binding ledger; the scalar twin of
  /// [`Expr::render_into`].
  #[must_use]
  pub fn render_into(&self, params: &mut BoundParams, dialect: Dialect) -> String {
    render_scalar(&self.kind, params, dialect)
  }

  /// Render with dialect-specific placeholders starting at `start`, returning
  /// the SQL and the values it binds, in emission order.
  #[must_use]
  pub fn to_sql_fragment_for(&self, start: usize, dialect: Dialect) -> (String, Vec<Value>) {
    let mut params = BoundParams::new();
    let sql = params.nested(start, |nested| self.render_into(nested, dialect));
    (sql, params.into_values())
  }

  /// [`Scalar::to_sql_fragment_for`] against [`Dialect::CURRENT`].
  #[must_use]
  pub fn to_sql_fragment(&self, start: usize) -> (String, Vec<Value>) {
    self.to_sql_fragment_for(start, Dialect::CURRENT)
  }
}
