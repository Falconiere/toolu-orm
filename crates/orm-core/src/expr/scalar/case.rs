//! `CASE WHEN … THEN … [ELSE …] END`.

use crate::expr::Expr;

use super::types::{Scalar, ScalarKind};

/// A `CASE` under construction; it always holds at least one `WHEN` branch,
/// so `CASE END` — which is not valid SQL — cannot be built.
///
/// Start one with [`Scalar::case_when`] and finish it with
/// [`CaseBuilder::otherwise`] or [`CaseBuilder::end`].
pub struct CaseBuilder {
  branches: Vec<(Expr, Scalar)>,
}

impl Scalar {
  /// Open a `CASE` with its first `WHEN <predicate> THEN <then>` branch.
  ///
  /// ```
  /// use toolu_orm_core::column::Integer;
  /// use toolu_orm_core::dialect::Dialect;
  /// use toolu_orm_core::expr::Scalar;
  /// use toolu_orm_core::query_column::{Column, NumericOps};
  ///
  /// const HITS: Column<Integer> = Column::new("memories", "access_count");
  ///
  /// let label = Scalar::case_when(HITS.gt(0), Scalar::bind("hot"))
  ///   .otherwise(Scalar::bind("cold"));
  /// let (sql, params) = label.to_sql_fragment_for(1, Dialect::Sqlite);
  /// assert_eq!(
  ///   sql,
  ///   r#"CASE WHEN "memories"."access_count" > ?1 THEN ?2 ELSE ?3 END"#
  /// );
  /// assert_eq!(params.len(), 3);
  /// ```
  #[must_use]
  pub fn case_when(predicate: Expr, then: Scalar) -> CaseBuilder {
    CaseBuilder {
      branches: vec![(predicate, then)],
    }
  }
}

impl CaseBuilder {
  /// Another `WHEN <predicate> THEN <then>` branch, tested after the ones
  /// already added.
  #[must_use]
  pub fn when(mut self, predicate: Expr, then: Scalar) -> Self {
    self.branches.push((predicate, then));
    self
  }

  /// Close the `CASE` with `ELSE <value> END`.
  #[must_use]
  pub fn otherwise(self, value: Scalar) -> Scalar {
    Scalar::from_kind(ScalarKind::Case {
      branches: self.branches,
      otherwise: Some(Box::new(value)),
    })
  }

  /// Close the `CASE` without an `ELSE`; SQL then yields `NULL` when no
  /// branch matches.
  #[must_use]
  pub fn end(self) -> Scalar {
    Scalar::from_kind(ScalarKind::Case {
      branches: self.branches,
      otherwise: None,
    })
  }
}
