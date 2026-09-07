//! Shared helpers for driver-specific `SelectBuilder` fetch implementations.

use crate::QueryError;

pub(crate) fn first_or_not_found<T>(results: Vec<T>, table: &str) -> Result<T, QueryError> {
  results
    .into_iter()
    .next()
    .ok_or_else(|| QueryError::NotFound {
      table: table.to_owned(),
    })
}

/// Generates async `fetch_all`, `fetch_one`, `fetch_optional`, `count`, `exists`
/// for `SelectBuilder` with a given scalar type for count/exists queries.
#[cfg(any(feature = "libsql", feature = "postgres"))]
macro_rules! impl_async_fetch {
  ($scalar_ty:ty) => {
    impl super::super::SelectBuilder {
      /// # Errors
      ///
      /// Returns [`QueryError`] when the underlying query or row mapping fails.
      pub async fn fetch_all<T: toolu_orm_core::row::FromRow + Send>(
        &self,
        exec: &(impl $crate::executor::Executor + Send + Sync),
      ) -> Result<Vec<T>, $crate::QueryError> {
        let (sql, params) = self.to_sql();
        exec.query_map::<T>(&sql, params).await
      }

      /// # Errors
      ///
      /// Returns [`QueryError`] when the query fails, mapping fails, or no row is found.
      pub async fn fetch_one<T: toolu_orm_core::row::FromRow + Send>(
        &self,
        exec: &(impl $crate::executor::Executor + Send + Sync),
      ) -> Result<T, $crate::QueryError> {
        let results = self.fetch_all::<T>(exec).await?;
        super::shared::first_or_not_found(results, &self.table)
      }

      /// # Errors
      ///
      /// Returns [`QueryError`] when the query or row mapping fails.
      pub async fn fetch_optional<T: toolu_orm_core::row::FromRow + Send>(
        &self,
        exec: &(impl $crate::executor::Executor + Send + Sync),
      ) -> Result<Option<T>, $crate::QueryError> {
        let results = self.fetch_all::<T>(exec).await?;
        Ok(results.into_iter().next())
      }

      /// # Errors
      ///
      /// Returns [`QueryError`] when the count query fails or the scalar row is missing.
      pub async fn count(
        &self,
        exec: &(impl $crate::executor::Executor + Send + Sync),
      ) -> Result<i64, $crate::QueryError> {
        let (sql, params) = self.to_count_sql();
        let rows = exec.query_map::<$scalar_ty>(&sql, params).await?;
        super::shared::first_or_not_found(rows, &self.table).map(|r| r.value)
      }

      /// # Errors
      ///
      /// Returns [`QueryError`] when the exists query or scalar mapping fails.
      pub async fn exists(
        &self,
        exec: &(impl $crate::executor::Executor + Send + Sync),
      ) -> Result<bool, $crate::QueryError> {
        let (sql, params) = self.to_exists_sql();
        let rows = exec.query_map::<$scalar_ty>(&sql, params).await?;
        Ok(
          rows
            .into_iter()
            .next()
            .map(|r| r.value != 0)
            .unwrap_or(false),
        )
      }
    }
  };
}

#[cfg(any(feature = "libsql", feature = "postgres"))]
pub(super) use impl_async_fetch;

/// Generates sync `fetch_all`, `fetch_one`, `fetch_optional`, `count`, `exists`
/// for `SelectBuilder` with a given scalar type (rusqlite path).
#[cfg(feature = "rusqlite")]
macro_rules! impl_sync_fetch {
  ($scalar_ty:ty) => {
    impl super::super::SelectBuilder {
      /// # Errors
      ///
      /// Returns [`QueryError`] when the underlying query or row mapping fails.
      pub fn fetch_all<T: toolu_orm_core::row::FromRow>(
        &self,
        exec: &impl $crate::executor::Executor,
      ) -> Result<Vec<T>, $crate::QueryError> {
        let (sql, params) = self.to_sql();
        exec.query_map::<T>(&sql, params)
      }

      /// # Errors
      ///
      /// Returns [`QueryError`] when the query fails, mapping fails, or no row is found.
      pub fn fetch_one<T: toolu_orm_core::row::FromRow>(
        &self,
        exec: &impl $crate::executor::Executor,
      ) -> Result<T, $crate::QueryError> {
        let results = self.fetch_all::<T>(exec)?;
        super::shared::first_or_not_found(results, &self.table)
      }

      /// # Errors
      ///
      /// Returns [`QueryError`] when the query or row mapping fails.
      pub fn fetch_optional<T: toolu_orm_core::row::FromRow>(
        &self,
        exec: &impl $crate::executor::Executor,
      ) -> Result<Option<T>, $crate::QueryError> {
        let results = self.fetch_all::<T>(exec)?;
        Ok(results.into_iter().next())
      }

      /// # Errors
      ///
      /// Returns [`QueryError`] when the count query fails or the scalar row is missing.
      pub fn count(
        &self,
        exec: &impl $crate::executor::Executor,
      ) -> Result<i64, $crate::QueryError> {
        let (sql, params) = self.to_count_sql();
        let rows = exec.query_map::<$scalar_ty>(&sql, params)?;
        super::shared::first_or_not_found(rows, &self.table).map(|r| r.value)
      }

      /// # Errors
      ///
      /// Returns [`QueryError`] when the exists query or scalar mapping fails.
      pub fn exists(
        &self,
        exec: &impl $crate::executor::Executor,
      ) -> Result<bool, $crate::QueryError> {
        let (sql, params) = self.to_exists_sql();
        let rows = exec.query_map::<$scalar_ty>(&sql, params)?;
        Ok(
          rows
            .into_iter()
            .next()
            .map(|r| r.value != 0)
            .unwrap_or(false),
        )
      }
    }
  };
}

#[cfg(feature = "rusqlite")]
pub(super) use impl_sync_fetch;
