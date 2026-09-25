//! `WHERE` and `HAVING` clause rendering, and dialect-aware parameter
//! management.

use toolu_orm_core::alias::quote_ident;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::{BoundParams, Expr};

/// Appends a `WHERE` clause to `sql` using dialect-specific parameters.
pub(crate) fn append_where_for(
  filters: &[Expr],
  sql: &mut String,
  params: &mut BoundParams,
  dialect: Dialect,
) {
  append_conjuncts_for(filters, " WHERE ", sql, params, dialect);
}

/// `<keyword><conjunct>[ AND <conjunct>]…`, or nothing when `exprs` is empty.
///
/// `WHERE` and `HAVING` differ only in the keyword: both are an `AND`-joined
/// list of predicates, each numbering from the parameters already emitted.
/// One renderer is what keeps the two from drifting apart.
pub(crate) fn append_conjuncts_for(
  exprs: &[Expr],
  keyword: &str,
  sql: &mut String,
  params: &mut BoundParams,
  dialect: Dialect,
) {
  if exprs.is_empty() {
    return;
  }

  sql.push_str(keyword);
  let mut first = true;
  for expr in exprs {
    let fragment = expr.render_into(params, dialect);
    if !first {
      sql.push_str(" AND ");
    }
    sql.push_str(&fragment);
    first = false;
  }
}

/// Appends ` RETURNING "a", "b"` when `columns` is non-empty.
///
/// The list is unqualified, which both engines accept, and it binds nothing.
pub(crate) fn append_returning(columns: &[String], sql: &mut String) {
  if columns.is_empty() {
    return;
  }
  let cols: Vec<String> = columns.iter().map(|c| quote_ident(c)).collect();
  sql.push_str(&format!(" RETURNING {}", cols.join(", ")));
}

// ── impl_filter ───────────────────────────────────────────────────────────────

/// Generates a `filter(self, expr: Expr) -> Self` method for a builder struct
/// that has a `filters: Vec<Expr>` field.
macro_rules! impl_filter {
  ($ty:ty) => {
    impl $ty {
      /// Adds a predicate to this query.
      pub fn filter(mut self, expr: toolu_orm_core::expr::Expr) -> Self {
        self.filters.push(expr);
        self
      }
    }
  };
}

pub(crate) use impl_filter;

/// Wraps items when one implemented backend and no Lance feature is active.
///
/// Used for modules and imports that require exactly one backend feature active.
macro_rules! cfg_single_backend {
  ($($item:item)*) => {
    $(
      #[cfg(all(
        not(feature = "lancedb"),
        any(
          all(feature = "libsql", not(feature = "rusqlite"), not(feature = "postgres")),
          all(feature = "rusqlite", not(feature = "libsql"), not(feature = "postgres")),
          all(feature = "postgres", not(feature = "libsql"), not(feature = "rusqlite")),
        )
      ))]
      $item
    )*
  };
}

pub(crate) use cfg_single_backend;
