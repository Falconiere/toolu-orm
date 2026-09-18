//! Expression AST for WHERE clause generation across dialects.

mod fts5_match;
pub(crate) mod function_name;
mod pg_fts_match;
mod render;
mod scalar;
mod subquery;
mod types;

pub use scalar::{like_pattern_literal, CaseBuilder, Scalar};
pub use subquery::SelectSource;
pub use types::{Expr, JoinCondition, JsonExpr, OrderBy};
