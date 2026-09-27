//! Explicit refusal of a general Lance transaction before any SQL is executed.

use super::LanceDbConnection;
use crate::{Capability, DbError};
use toolu_orm_core::dialect::Dialect;

impl LanceDbConnection {
  /// Refuse a general multi-statement transaction on the Lance session.
  ///
  /// Narrow DML rollback observations do not establish support for arbitrary
  /// transactions (in particular DDL). This entry executes no SQL and takes no
  /// lock. There is no successful transaction handle in the portability contract.
  ///
  /// # Errors
  /// Always returns [`DbError::UnsupportedCapability`] naming
  /// [`Capability::MultiStatementTransaction`].
  pub async fn begin(&self) -> Result<std::convert::Infallible, DbError> {
    Err(DbError::UnsupportedCapability {
      backend: Dialect::Lance,
      capability: Capability::MultiStatementTransaction,
    })
  }
}
