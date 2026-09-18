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
use crate::expr::BoundParams;
use crate::value::Value;

/// A complete `SELECT` statement whose first placeholder takes a
/// caller-chosen index.
///
/// The returned SQL carries no surrounding parentheses, so each caller adds
/// what its own position needs — `IN (…)`, `EXISTS (…)` and a scalar
/// `(SELECT …)` parenthesise, an `INSERT … SELECT` appends it bare. Splice
/// with `to_select_sql_into(params.next_index(), params, dialect)`: position
/// lives in the buffer, which is the numbering rule every other node here
/// follows.
///
/// `Send + Sync` are supertraits so `Box<dyn SelectSource>` stays `Send` and
/// `Sync`, which keeps `Expr`, `Scalar` and the builders usable across an
/// `await`.
pub trait SelectSource: Send + Sync {
  /// The statement and the values it binds, first placeholder at `start`.
  fn to_select_sql_for(&self, start: usize, dialect: Dialect) -> (String, Vec<Value>);

  /// The statement rendered into `params`, sharing its binding ledger so a
  /// [`SharedBind`](crate::expr::SharedBind) used inside *and* outside this
  /// statement takes one placeholder.
  ///
  /// `start` is where the caller will place this statement's first value, and
  /// every splice in this crate passes [`BoundParams::next_index`]; passing
  /// anything else would number placeholders away from where the values land.
  ///
  /// The default renders through [`Self::to_select_sql_for`] with an
  /// independent ledger: correct SQL and correct values, but a handle used on
  /// both sides binds once on each. Implementors that render into a
  /// [`BoundParams`] should override it — `SelectBuilder` does, through
  /// [`BoundParams::nested`].
  ///
  /// **Do not implement [`Self::to_select_sql_for`] by calling this method
  /// while leaving this method defaulted**: the default calls
  /// `to_select_sql_for`, so the pair would recurse without end. Give both a
  /// common private body, the way `SelectBuilder` does.
  fn to_select_sql_into(&self, start: usize, params: &mut BoundParams, dialect: Dialect) -> String {
    let (sql, sub_params) = self.to_select_sql_for(start, dialect);
    params.extend(sub_params);
    sql
  }
}
