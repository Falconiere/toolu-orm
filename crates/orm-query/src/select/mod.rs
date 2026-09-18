//! SELECT query builder.
//!
//! # Public API
//!
//! - [`SelectBuilder`] — fluent builder for SELECT queries
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
mod count_exists;
mod grouping;
mod join_clause;
mod knn;
mod ordering;
mod projection;
pub mod relational;
mod row_limit;
mod statement;

use crate::where_clause::cfg_single_backend;

cfg_single_backend! {
  mod executor_fetch;
}

pub use builder::SelectBuilder;
pub use relational::{RelationColumn, RelationConfig, RelationalSelectBuilder};
