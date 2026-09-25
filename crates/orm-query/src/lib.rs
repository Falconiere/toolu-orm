/// DELETE query builder.
pub mod delete;
/// Query execution errors.
pub mod error;
/// INSERT query builder.
pub mod insert;
/// Relation-aware query builder.
pub mod relational_builder;
/// SELECT query builder.
pub mod select;
/// Tuple append support for generated builders.
pub mod tuple_append;
/// UPDATE query builder.
pub mod update;
/// Shared WHERE clause rendering and backend gates.
pub(crate) mod where_clause;

use where_clause::cfg_single_backend;

cfg_single_backend! {
  /// Executor for the active implemented backend.
  pub mod executor;
  /// Helpers shared by active backend executors.
  pub(crate) mod exec_helpers;
}

#[cfg(all(
  feature = "libsql",
  not(feature = "rusqlite"),
  not(feature = "postgres")
))]
#[cfg(not(feature = "lancedb"))]
/// Libsql transaction wrapper for the sole active backend.
pub mod transaction;

pub use error::QueryError;
