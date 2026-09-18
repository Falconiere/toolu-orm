//! The table-valued half of a [`TableRef`](super::TableRef): what a `FROM`
//! slot names when it is a call rather than a relation.

use crate::dialect::Dialect;
use crate::value::Value;

/// What a `FROM` / `JOIN` slot actually names.
///
/// A schema table and a common table expression are both `Relation` — a CTE is
/// referenced by a plain quoted identifier, so it needs nothing of its own.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum TableSource {
  /// One quoted identifier.
  Relation,
  /// `name(?1, …)` and the values its arguments bind.
  ///
  /// `Vec<Value>` rather than a scalar tree: a table-valued call takes
  /// *arguments*, and keeping them values is what lets `TableRef` stay `Clone`
  /// — which `impl From<&TableRef> for TableRef` depends on. It costs the
  /// derived `Eq`, since `Value::Real` holds an `f64`.
  Function(Vec<Value>),
}

/// `name(?N, ?N+1, …)`, numbering from `start`, with the values in bind order.
///
/// The name is not quoted — it is SQL syntax, and `is_valid_function_name`
/// has already refused anything that is not a bare identifier.
pub(super) fn render_call(
  name: &str,
  args: &[Value],
  start: usize,
  dialect: Dialect,
) -> (String, Vec<Value>) {
  let placeholders: Vec<String> = (0..args.len()).map(|i| dialect.param(start + i)).collect();
  (
    format!("{name}({})", placeholders.join(", ")),
    args.to_vec(),
  )
}
