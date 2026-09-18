//! [`SelectSource`] — a whole `SELECT` statement that another statement can
//! hold.
//!
//! The builder that renders a `SELECT` lives in `toolu-orm-query`, which
//! depends on this crate, so a subquery node cannot name it directly. Nor can
//! it hold pre-rendered SQL: [`Expr::raw`](crate::expr::Expr::raw) passes an
//! already-numbered `?N` through unchanged, so a statement rendered in advance
//! would keep the indices it was born with. An object-safe trait here,
//! implemented over there, is how [`QualifiedColumn`](crate::alias::QualifiedColumn)
//! already solves this for columns.

use crate::dialect::Dialect;
use crate::value::Value;

/// A complete `SELECT` statement whose first placeholder takes a
/// caller-chosen index.
///
/// The returned SQL carries no surrounding parentheses, so each caller adds
/// what its own position needs — `IN (…)`, `EXISTS (…)` and a scalar
/// `(SELECT …)` parenthesise, an `INSERT … SELECT` appends it bare. Splice
/// with `to_select_sql_for(start + params.len(), dialect)` and extend, the
/// numbering rule every other node here follows.
///
/// `Send + Sync` are supertraits so `Box<dyn SelectSource>` stays `Send` and
/// `Sync`, which keeps `Expr`, `Scalar` and the builders usable across an
/// `await`.
pub trait SelectSource: Send + Sync {
  /// The statement and the values it binds, first placeholder at `start`.
  fn to_select_sql_for(&self, start: usize, dialect: Dialect) -> (String, Vec<Value>);
}
