//! Aggregate calls: `COUNT(*)`, `COUNT(DISTINCT x)`, `SUM`, `MAX`, `MIN`, `AVG`.

use super::types::{AggregateArg, Scalar, ScalarKind};

impl Scalar {
  /// `COUNT(*)` — every row in the group, NULL-only rows included.
  ///
  /// One of the two shapes [`Scalar::func`] cannot express: `*` is not a value
  /// expression, so it is not an argument.
  #[must_use]
  pub fn count_star() -> Self {
    Self::aggregate("COUNT", AggregateArg::Star)
  }

  /// `COUNT(<arg>)` — the rows of the group where `arg` is not NULL.
  #[must_use]
  pub fn count(arg: Scalar) -> Self {
    Self::aggregate("COUNT", AggregateArg::All(Box::new(arg)))
  }

  /// `COUNT(DISTINCT <arg>)` — how many *different* non-NULL values the group
  /// holds.
  ///
  /// The other shape [`Scalar::func`] cannot express: `DISTINCT` is a keyword
  /// inside the argument list, not an argument.
  #[must_use]
  pub fn count_distinct(arg: Scalar) -> Self {
    Self::aggregate("COUNT", AggregateArg::Distinct(Box::new(arg)))
  }

  /// `SUM(<arg>)`; NULL over an empty group, so decode it as an `Option`.
  #[must_use]
  pub fn sum(arg: Scalar) -> Self {
    Self::aggregate("SUM", AggregateArg::All(Box::new(arg)))
  }

  /// `MAX(<arg>)`; NULL over an empty group.
  #[must_use]
  pub fn max(arg: Scalar) -> Self {
    Self::aggregate("MAX", AggregateArg::All(Box::new(arg)))
  }

  /// `MIN(<arg>)`; NULL over an empty group.
  #[must_use]
  pub fn min(arg: Scalar) -> Self {
    Self::aggregate("MIN", AggregateArg::All(Box::new(arg)))
  }

  /// `AVG(<arg>)`; NULL over an empty group.
  ///
  /// SQLite returns `REAL`. Postgres returns `numeric` for an integer
  /// argument, which the row decoders do not map to a Rust float — cast it in
  /// the projection, or read it on SQLite only.
  #[must_use]
  pub fn avg(arg: Scalar) -> Self {
    Self::aggregate("AVG", AggregateArg::All(Box::new(arg)))
  }

  fn aggregate(func: &'static str, arg: AggregateArg) -> Self {
    Self::from_kind(ScalarKind::Aggregate { func, arg })
  }
}
