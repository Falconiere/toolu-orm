//! Transaction-scoped session settings for row-level security.
//!
//! A policy usually reads its context from a setting —
//! `current_setting('app.tenant_id')` — that the application has to put in
//! place before the query runs. `SET LOCAL` cannot take bind parameters, so
//! writing it by hand means formatting a value into SQL text on the one code
//! path that is supposed to enforce isolation. [`PgTransaction::set_local_config`]
//! goes through `set_config($1, $2, true)` instead: both the name and the
//! value are bound, and `true` scopes the setting to the transaction, so it is
//! gone when the connection returns to the pool.

use crate::error::DbError;
use crate::trait_def::DbConnection;
use toolu_orm_core::value::Value;

use super::connection::PgTransaction;

/// `set_config(name, value, true)`, with both arguments bound.
const SET_LOCAL_CONFIG: &str = "SELECT set_config($1, $2, true)";

impl PgTransaction<'_> {
  /// Sets a configuration parameter for the rest of this transaction.
  ///
  /// `name` must be a two-part custom name such as `app.tenant_id`, or a
  /// built-in parameter the role may set; Postgres rejects anything else.
  /// The value is reset when the transaction commits or rolls back.
  ///
  /// # Errors
  ///
  /// Returns `DbError::Query` when Postgres rejects the name or the value.
  pub async fn set_local_config(&self, name: &str, value: &str) -> Result<(), DbError> {
    self
      .execute_sql(
        SET_LOCAL_CONFIG,
        vec![Value::Text(name.to_owned()), Value::Text(value.to_owned())],
      )
      .await?;
    Ok(())
  }
}
