//! `name(arg, ...)` — a scalar function call with a validated name.

use crate::error::DbCoreError;
use crate::expr::function_name::is_valid_function_name;

use super::types::{Scalar, ScalarKind};

impl Scalar {
  /// A function call: `datetime(?1)`, `coalesce("a", "b")`, `substr(…, 1, 8)`.
  ///
  /// The name is *validated* rather than escaped, the way FTS5 table names
  /// are: it names SQL syntax, so anything outside `[A-Za-z_][A-Za-z0-9_]*`
  /// is a caller mistake and never reaches a statement. Arguments are
  /// scalars, so they carry their own binds.
  ///
  /// No cross-dialect translation happens here: `datetime` is SQLite's and
  /// `to_timestamp` is Postgres's, and naming one for the other dialect is
  /// the caller's error, exactly as with [`crate::expr::Expr::raw`].
  ///
  /// ```
  /// use toolu_orm_core::dialect::Dialect;
  /// use toolu_orm_core::expr::Scalar;
  ///
  /// # fn main() -> Result<(), toolu_orm_core::error::DbCoreError> {
  /// let cutoff = Scalar::func("datetime", vec![Scalar::bind("2026-09-18T10:00:00Z")])?;
  /// let (sql, params) = cutoff.to_sql_fragment_for(2, Dialect::Postgres);
  /// assert_eq!(sql, "datetime($2)");
  /// assert_eq!(params.len(), 1);
  /// assert!(Scalar::func("drop table users; --", Vec::new()).is_err());
  /// # Ok(())
  /// # }
  /// ```
  ///
  /// # Errors
  ///
  /// [`DbCoreError::InvalidScalarFunction`] when `name` is empty or holds
  /// anything but ASCII letters, digits and underscores, or starts with a
  /// digit.
  pub fn func(name: &str, args: Vec<Scalar>) -> Result<Self, DbCoreError> {
    if !is_valid_function_name(name) {
      return Err(DbCoreError::InvalidScalarFunction {
        name: name.to_owned(),
      });
    }
    Ok(Self::from_kind(ScalarKind::Func {
      name: name.to_owned(),
      args,
    }))
  }
}
