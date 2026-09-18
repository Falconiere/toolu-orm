//! `ORDER BY <scalar>` terms.

use crate::expr::OrderBy;

use super::types::Scalar;

impl Scalar {
  /// `ORDER BY <self> ASC`.
  #[must_use]
  pub fn asc(self) -> OrderBy {
    OrderBy::from_term(self, "ASC")
  }

  /// `ORDER BY <self> DESC`.
  #[must_use]
  pub fn desc(self) -> OrderBy {
    OrderBy::from_term(self, "DESC")
  }
}
