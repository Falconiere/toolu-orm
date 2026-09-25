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
        let (sql, params) = self.to_sql_for(exec.dialect());
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
        let (sql, params) = self.to_sql_for(exec.dialect());
        exec.execute_sql(&sql, params)
      }
    }
  };
}

pub(crate) use impl_execute;

/// Generates feature-gated `fetch_all` / `fetch_optional` / `fetch_one` for a
/// write builder that projects rows with `RETURNING`.
///
/// They go through the same `Executor::query_map` every driver implements for
/// `SelectBuilder`, and share its contract: a statement that returns no row —
/// no `RETURNING` clause, or a conflict clause that took the `DO NOTHING`
/// branch — makes `fetch_optional` `None` and `fetch_one`
/// `QueryError::NotFound`. The builder must expose a `table_name()` method for
/// that error, so a target carrying a database qualifier still reports the
/// bare table it named.
macro_rules! impl_returning_fetch {
  ($builder:ty, $op:literal) => {
    #[cfg(any(
      all(feature = "libsql", not(feature = "rusqlite"), not(feature = "postgres")),
      all(feature = "postgres", not(feature = "libsql"), not(feature = "rusqlite")),
    ))]
    impl $builder {
      #[doc = concat!("Every row this ", $op, " projects with `RETURNING`.")]
      ///
      /// # Errors
      ///
      /// Returns `QueryError::Driver` on database errors and
      /// `QueryError::RowMapping` when a projected row does not decode.
      pub async fn fetch_all<T: toolu_orm_core::row::FromRow + Send>(
        &self,
        exec: &(impl $crate::executor::Executor + Send + Sync),
      ) -> Result<Vec<T>, $crate::QueryError> {
        let (sql, params) = self.to_sql_for(exec.dialect());
        exec.query_map::<T>(&sql, params).await
      }

      #[doc = concat!("The first row this ", $op, " projects, or `None`.")]
      ///
      /// # Errors
      ///
      /// As [`Self::fetch_all`].
      pub async fn fetch_optional<T: toolu_orm_core::row::FromRow + Send>(
        &self,
        exec: &(impl $crate::executor::Executor + Send + Sync),
      ) -> Result<Option<T>, $crate::QueryError> {
        Ok(self.fetch_all::<T>(exec).await?.into_iter().next())
      }

      #[doc = concat!("The first row this ", $op, " projects.")]
      ///
      /// # Errors
      ///
      /// As [`Self::fetch_all`], plus `QueryError::NotFound` when the
      /// statement projected no row at all.
      pub async fn fetch_one<T: toolu_orm_core::row::FromRow + Send>(
        &self,
        exec: &(impl $crate::executor::Executor + Send + Sync),
      ) -> Result<T, $crate::QueryError> {
        self
          .fetch_optional::<T>(exec)
          .await?
          .ok_or_else(|| $crate::QueryError::NotFound {
            table: self.table_name().to_owned(),
          })
      }
    }

    #[cfg(all(
      feature = "rusqlite",
      not(feature = "libsql"),
      not(feature = "postgres")
    ))]
    impl $builder {
      #[doc = concat!("Every row this ", $op, " projects with `RETURNING`.")]
      ///
      /// # Errors
      ///
      /// Returns `QueryError::Driver` on database errors and
      /// `QueryError::RowMapping` when a projected row does not decode.
      pub fn fetch_all<T: toolu_orm_core::row::FromRow>(
        &self,
        exec: &impl $crate::executor::Executor,
      ) -> Result<Vec<T>, $crate::QueryError> {
        let (sql, params) = self.to_sql_for(exec.dialect());
        exec.query_map::<T>(&sql, params)
      }

      #[doc = concat!("The first row this ", $op, " projects, or `None`.")]
      ///
      /// # Errors
      ///
      /// As [`Self::fetch_all`].
      pub fn fetch_optional<T: toolu_orm_core::row::FromRow>(
        &self,
        exec: &impl $crate::executor::Executor,
      ) -> Result<Option<T>, $crate::QueryError> {
        Ok(self.fetch_all::<T>(exec)?.into_iter().next())
      }

      #[doc = concat!("The first row this ", $op, " projects.")]
      ///
      /// # Errors
      ///
      /// As [`Self::fetch_all`], plus `QueryError::NotFound` when the
      /// statement projected no row at all.
      pub fn fetch_one<T: toolu_orm_core::row::FromRow>(
        &self,
        exec: &impl $crate::executor::Executor,
      ) -> Result<T, $crate::QueryError> {
        self
          .fetch_optional::<T>(exec)?
          .ok_or_else(|| $crate::QueryError::NotFound {
            table: self.table_name().to_owned(),
          })
      }
    }
  };
}

pub(crate) use impl_returning_fetch;
