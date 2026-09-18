//! Rendering for the nodes that hold a whole statement.
//!
//! Each splices the nested statement at `start + params.len()` — the index its
//! siblings have already reached — and extends the parameter vector with what
//! the statement bound, so numbering continues across the boundary.

use crate::dialect::Dialect;
use crate::expr::scalar::Scalar;
use crate::expr::SelectSource;
use crate::value::Value;

use super::scalar::render_scalar;

/// `EXISTS (<query>)` / `NOT EXISTS (<query>)`.
pub(super) fn render_exists(
  query: &dyn SelectSource,
  negated: bool,
  start: usize,
  params: &mut Vec<Value>,
  dialect: Dialect,
) -> String {
  let keyword = if negated { "NOT EXISTS" } else { "EXISTS" };
  format!("{keyword} ({})", splice(query, start, params, dialect))
}

/// `<left> IN (<query>)` / `<left> NOT IN (<query>)`.
///
/// The left scalar renders first, so its own placeholders take the lower
/// indices — the order the SQL reads in.
pub(super) fn render_in_subquery(
  left: &Scalar,
  query: &dyn SelectSource,
  negated: bool,
  start: usize,
  params: &mut Vec<Value>,
  dialect: Dialect,
) -> String {
  let left_sql = render_scalar(&left.kind, start, params, dialect);
  let keyword = if negated { "NOT IN" } else { "IN" };
  format!(
    "{left_sql} {keyword} ({})",
    splice(query, start, params, dialect)
  )
}

/// `(SELECT …)` in value position.
pub(super) fn render_scalar_subquery(
  query: &dyn SelectSource,
  start: usize,
  params: &mut Vec<Value>,
  dialect: Dialect,
) -> String {
  format!("({})", splice(query, start, params, dialect))
}

/// The nested statement, numbered from where its siblings left off.
fn splice(
  query: &dyn SelectSource,
  start: usize,
  params: &mut Vec<Value>,
  dialect: Dialect,
) -> String {
  let (sql, sub_params) = query.to_select_sql_for(start + params.len(), dialect);
  params.extend(sub_params);
  sql
}
