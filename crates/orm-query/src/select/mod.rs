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
pub mod relational;

use crate::where_clause::cfg_single_backend;

cfg_single_backend! {
  mod executor_fetch;
}

pub use builder::SelectBuilder;
pub use relational::{RelationConfig, RelationalSelectBuilder};
