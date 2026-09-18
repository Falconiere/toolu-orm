//! Rendering for the nodes that hold a whole statement.
//!
//! Each splices the nested statement at [`BoundParams::next_index`] — the index
//! its siblings have already reached — and renders into the same buffer, so
//! numbering *and* the shared-binding ledger continue across the boundary.

use crate::dialect::Dialect;
use crate::expr::scalar::Scalar;
use crate::expr::{BoundParams, SelectSource};

use super::scalar::render_scalar;

/// `EXISTS (<query>)` / `NOT EXISTS (<query>)`.
pub(super) fn render_exists(
  query: &dyn SelectSource,
  negated: bool,
  params: &mut BoundParams,
  dialect: Dialect,
) -> String {
  let keyword = if negated { "NOT EXISTS" } else { "EXISTS" };
  format!("{keyword} ({})", splice(query, params, dialect))
}

/// `<left> IN (<query>)` / `<left> NOT IN (<query>)`.
///
/// The left scalar renders first, so its own placeholders take the lower
/// indices — the order the SQL reads in.
pub(super) fn render_in_subquery(
  left: &Scalar,
  query: &dyn SelectSource,
  negated: bool,
  params: &mut BoundParams,
  dialect: Dialect,
) -> String {
  let left_sql = render_scalar(&left.kind, params, dialect);
  let keyword = if negated { "NOT IN" } else { "IN" };
  format!("{left_sql} {keyword} ({})", splice(query, params, dialect))
}

/// `(SELECT …)` in value position.
pub(super) fn render_scalar_subquery(
  query: &dyn SelectSource,
  params: &mut BoundParams,
  dialect: Dialect,
) -> String {
  format!("({})", splice(query, params, dialect))
}

/// The nested statement, numbered from where its siblings left off.
///
/// `next_index()` is read **at the moment the statement renders**, so this is
/// correct for any buffer — empty in `render_exists`, already holding the left
/// scalar's binds in `render_in_subquery`, or holding anything a future arm
/// renders first. That is the rule every node in this module follows; see
/// [`render_scalar`](super::render_scalar).
fn splice(query: &dyn SelectSource, params: &mut BoundParams, dialect: Dialect) -> String {
  query.to_select_sql_into(params.next_index(), params, dialect)
}
