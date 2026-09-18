//! Expression AST for WHERE clause generation across dialects.

mod fts5_match;
mod pg_fts_match;
mod render;
mod scalar;
mod types;

pub use scalar::{like_pattern_literal, CaseBuilder, Scalar};
pub use types::{Expr, JoinCondition, JsonExpr, OrderBy};
