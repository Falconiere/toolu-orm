//! Write execution through the connection-selected dialect and parameter codec.

use toolu_orm_connection::{DbConnection, DbError};

use crate::{delete::DeleteBuilder, insert::InsertBuilder, update::UpdateBuilder};

macro_rules! impl_portable_execute {
  ($builder:ty) => {
    impl $builder {
      /// Execute using the connection's dialect and return its affected-row count.
      ///
      /// This async entry is available in single-driver and mixed-driver builds,
      /// including Lance. Parameters are converted by the selected connection.
      /// It does not fetch `RETURNING` rows or emulate unsupported capabilities.
      ///
      /// # Errors
      ///
      /// Propagates the connection's [`DbError`] unchanged for binding, database,
      /// or connection failures. A failed operation never returns an affected count.
      pub async fn execute_on(&self, conn: &impl DbConnection) -> Result<u64, DbError> {
        let (sql, params) = self.to_sql_for(conn.dialect());
        conn.execute_sql(&sql, params).await
      }
    }
  };
}

impl_portable_execute!(InsertBuilder);
impl_portable_execute!(UpdateBuilder);
impl_portable_execute!(DeleteBuilder);
