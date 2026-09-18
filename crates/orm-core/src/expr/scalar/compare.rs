//! Comparisons between two scalars, including `LIKE` with an escape character.

use crate::expr::Expr;

use super::types::Scalar;

impl Scalar {
  /// `self = other`.
  #[must_use]
  pub fn eq(self, other: Scalar) -> Expr {
    Expr::compare(self, "=", other)
  }

  /// `self != other`.
  #[must_use]
  pub fn ne(self, other: Scalar) -> Expr {
    Expr::compare(self, "!=", other)
  }

  /// `self > other`.
  #[must_use]
  pub fn gt(self, other: Scalar) -> Expr {
    Expr::compare(self, ">", other)
  }

  /// `self >= other`.
  ///
  /// ```
  /// use toolu_orm_core::column::Text;
  /// use toolu_orm_core::dialect::Dialect;
  /// use toolu_orm_core::expr::Scalar;
  /// use toolu_orm_core::query_column::Column;
  ///
  /// const CREATED_AT: Column<Text> = Column::new("memories", "created_at");
  ///
  /// # fn main() -> Result<(), toolu_orm_core::error::DbCoreError> {
  /// let since = Scalar::func("datetime", vec![Scalar::bind("2026-09-18T10:00:00Z")])?;
  /// let stamp = Scalar::func("datetime", vec![Scalar::col(&CREATED_AT)])?;
  /// let (sql, params) = stamp.gte(since).to_sql_fragment_for(1, Dialect::Sqlite);
  /// assert_eq!(sql, r#"datetime("memories"."created_at") >= datetime(?1)"#);
  /// assert_eq!(params.len(), 1);
  /// # Ok(())
  /// # }
  /// ```
  #[must_use]
  pub fn gte(self, other: Scalar) -> Expr {
    Expr::compare(self, ">=", other)
  }

  /// `self < other`.
  #[must_use]
  pub fn lt(self, other: Scalar) -> Expr {
    Expr::compare(self, "<", other)
  }

  /// `self <= other`.
  #[must_use]
  pub fn lte(self, other: Scalar) -> Expr {
    Expr::compare(self, "<=", other)
  }

  /// `self LIKE pattern`, with `%` and `_` in `pattern` acting as wildcards.
  #[must_use]
  pub fn like(self, pattern: Scalar) -> Expr {
    Expr::like_node(self, pattern, None)
  }

  /// `self LIKE pattern ESCAPE ?`, so a `%` or `_` the pattern prefixes with
  /// `escape` matches literally.
  ///
  /// The escape character is *bound*, not interpolated, and a Rust `char` is
  /// one character by construction — which is what both engines require of
  /// this operand. Build the pattern with
  /// [`like_pattern_literal`](crate::expr::like_pattern_literal).
  #[must_use]
  pub fn like_escape(self, pattern: Scalar, escape: char) -> Expr {
    Expr::like_node(self, pattern, Some(escape))
  }
}
