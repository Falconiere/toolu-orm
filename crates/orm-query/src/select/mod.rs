//! SELECT query builder.
//!
//! # Public API
//!
//! - [`SelectBuilder`] — fluent builder for SELECT queries
//! - [`Cte`] — one member of a `WITH` / `WITH RECURSIVE` prefix
//!
//! # Usage
//!
//! ```ignore
//! let (sql, params) = SelectBuilder::new("users")
//!     .columns_raw(&["id", "name"])
//!     .limit(10)
//!     .to_sql();
//! ```

mod builder;
mod compound;
mod count_exists;
mod cte;
mod grouping;
mod join_clause;
mod knn;
mod ordering;
mod projection;
pub mod relational;
mod row_limit;
mod source;
mod statement;

use crate::where_clause::cfg_single_backend;

cfg_single_backend! {
  mod executor_fetch;
}

pub use builder::SelectBuilder;
pub use cte::Cte;
pub use relational::{RelationColumn, RelationConfig, RelationalSelectBuilder};
