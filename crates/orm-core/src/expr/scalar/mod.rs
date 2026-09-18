//! Scalar (value-producing) expression nodes.
//!
//! A [`Scalar`] is the half of the tree that produces a value rather than a
//! truth: a column, a bound value, a function call, arithmetic, string
//! concatenation, or a `CASE`. It composes into [`crate::expr::Expr`]
//! predicates and drops into `SELECT`, `ORDER BY`, `INSERT` values and
//! `UPDATE` assignments, all through the one parameter pipeline that numbers
//! placeholders from the count already emitted.

mod arith;
mod case;
mod compare;
mod excluded;
mod func;
mod like;
mod order;
mod types;

pub use case::CaseBuilder;
pub use like::like_pattern_literal;
pub use types::Scalar;

pub(crate) use types::ScalarKind;
