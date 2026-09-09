//! Postgres pgvector distance operators: `<->` / `<=>` / `<#>`.
//!
//! Separate from [`crate::vec0`]: sqlite-vec KNN (`MATCH` + hidden `k` +
//! synthesised `distance`) is a different model and is never translated here.
//! Also separate from [`crate::pg_fts`]. Every constructor requires
//! [`crate::dialect::Dialect::Postgres`] and refuses SQLite at construction
//! so no SQLite statement can carry these forms.

mod distance;
mod literal;
mod ops;
mod require;

pub use distance::{
  cosine_distance, cosine_distance_for, l2_distance, l2_distance_for, neg_inner_product,
  neg_inner_product_for, DistanceOp, PgVectorDistance,
};
pub use ops::PgVectorOps;
