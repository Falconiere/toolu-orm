//! WHERE clause builder and dialect-aware parameter management.

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::Expr;
use toolu_orm_core::value::Value;

/// Appends a `WHERE` clause to `sql` using dialect-specific parameters.
pub(crate) fn append_where_for(
  filters: &[Expr],
  sql: &mut String,
  params: &mut Vec<Value>,
  dialect: Dialect,
) {
  if filters.is_empty() {
    return;
  }

  sql.push_str(" WHERE ");
  let mut first = true;
  for filter in filters {
    let start = params.len() + 1;
    let (fragment, filter_params) = filter.to_sql_fragment_for(start, dialect);
    if !first {
      sql.push_str(" AND ");
    }
    sql.push_str(&fragment);
    params.extend(filter_params);
    first = false;
  }
}

// ── impl_filter ───────────────────────────────────────────────────────────────

/// Generates a `filter(self, expr: Expr) -> Self` method for a builder struct
/// that has a `filters: Vec<Expr>` field.
macro_rules! impl_filter {
  ($ty:ty) => {
    impl $ty {
      pub fn filter(mut self, expr: toolu_orm_core::expr::Expr) -> Self {
        self.filters.push(expr);
        self
      }
    }
  };
}

pub(crate) use impl_filter;

/// Wraps items with `#[cfg(any(single-libsql, single-rusqlite, single-postgres))]`.
///
/// Used for modules and imports that require exactly one backend feature active.
macro_rules! cfg_single_backend {
  ($($item:item)*) => {
    $(
      #[cfg(any(
        all(feature = "libsql", not(feature = "rusqlite"), not(feature = "postgres")),
        all(feature = "rusqlite", not(feature = "libsql"), not(feature = "postgres")),
        all(feature = "postgres", not(feature = "libsql"), not(feature = "rusqlite")),
      ))]
      $item
    )*
  };
}

pub(crate) use cfg_single_backend;
