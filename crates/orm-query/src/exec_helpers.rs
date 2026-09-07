//! Shared execution helper macros for write query builders.

/// Generates feature-gated `execute` methods for write builders
/// (InsertBuilder, UpdateBuilder, DeleteBuilder).
macro_rules! impl_execute {
  ($builder:ty, $op:literal) => {
    #[cfg(any(
      all(feature = "libsql", not(feature = "rusqlite"), not(feature = "postgres")),
      all(feature = "postgres", not(feature = "libsql"), not(feature = "rusqlite")),
    ))]
    impl $builder {
      #[doc = concat!("Execute this ", $op, " and return the number of affected rows.")]
      ///
      /// # Errors
      ///
      /// Returns `QueryError::Driver` on database errors.
      pub async fn execute(
        &self,
        exec: &(impl $crate::executor::Executor + Send + Sync),
      ) -> Result<u64, $crate::QueryError> {
        let (sql, params) = self.to_sql();
        exec.execute_sql(&sql, params).await
      }
    }

    #[cfg(all(
      feature = "rusqlite",
      not(feature = "libsql"),
      not(feature = "postgres")
    ))]
    impl $builder {
      #[doc = concat!("Execute this ", $op, " and return the number of affected rows.")]
      ///
      /// # Errors
      ///
      /// Returns `QueryError::Driver` on database errors.
      pub fn execute(
        &self,
        exec: &impl $crate::executor::Executor,
      ) -> Result<u64, $crate::QueryError> {
        let (sql, params) = self.to_sql();
        exec.execute_sql(&sql, params)
      }
    }
  };
}

pub(crate) use impl_execute;
