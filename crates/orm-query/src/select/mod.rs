//! SELECT builders, including [`SelectBuilder`](crate::select::SelectBuilder)
//! and [`Cte`](crate::select::Cte).

mod builder;
mod compound;
mod count_exists;
mod cte;
mod grouping;
mod join_clause;
mod knn;
mod ordering;
mod projection;
/// Relation-aware SELECT builder.
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
