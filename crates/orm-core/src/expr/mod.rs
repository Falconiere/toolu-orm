//! Expression AST for WHERE clause generation across dialects.

mod fts5_match;
mod pg_fts_match;
mod render;
mod types;

pub use types::{Expr, JoinCondition, JsonExpr, OrderBy};
