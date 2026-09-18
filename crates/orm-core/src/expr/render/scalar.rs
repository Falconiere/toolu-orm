//! Dialect-aware SQL rendering for [`ScalarKind`] trees.

use crate::dialect::Dialect;
use crate::expr::scalar::{Scalar, ScalarKind};
use crate::expr::Expr;
use crate::value::Value;

use super::predicate::render_expr;
use super::raw_params::number_raw_params;

/// Renders a scalar node, appending the values it binds to `params`.
///
/// `start` is the index the statement's *first* parameter takes — 1 for a
/// whole statement, higher for a fragment spliced after other clauses. It is
/// a constant for the whole tree: each binding arm adds the **live**
/// `params.len()` itself, at the moment it pushes, so a node numbers from
/// whatever its siblings already emitted however deeply it is nested.
///
/// Recursive calls therefore pass `start` through unchanged. Adding
/// `params.len()` at a call site would count the already-emitted parameters
/// twice; `render_expr` passes `start` to its `And` / `Or` children for the
/// same reason.
pub(crate) fn render_scalar(
  kind: &ScalarKind,
  start: usize,
  params: &mut Vec<Value>,
  dialect: Dialect,
) -> String {
  match kind {
    ScalarKind::Raw {
      sql,
      params: raw_params,
    } => {
      let base = start + params.len();
      params.extend(raw_params.iter().cloned());
      number_raw_params(sql, base, dialect)
    },
    ScalarKind::Bind(value) => {
      let idx = start + params.len();
      params.push(value.clone());
      dialect.param(idx)
    },
    ScalarKind::Func { name, args } => {
      let rendered: Vec<String> = args
        .iter()
        .map(|arg| render_scalar(&arg.kind, start, params, dialect))
        .collect();
      format!("{name}({})", rendered.join(", "))
    },
    ScalarKind::Arith { left, op, right } => {
      let left_sql = render_scalar(&left.kind, start, params, dialect);
      let right_sql = render_scalar(&right.kind, start, params, dialect);
      format!("({left_sql} {op} {right_sql})")
    },
    ScalarKind::Case {
      branches,
      otherwise,
    } => render_case(branches, otherwise.as_deref(), start, params, dialect),
  }
}

/// `CASE WHEN … THEN … [ELSE …] END`, each branch numbered after the last.
fn render_case(
  branches: &[(Expr, Scalar)],
  otherwise: Option<&Scalar>,
  start: usize,
  params: &mut Vec<Value>,
  dialect: Dialect,
) -> String {
  let mut sql = String::from("CASE");
  for (predicate, then) in branches {
    let when_sql = render_expr(&predicate.kind, start, params, dialect);
    let then_sql = render_scalar(&then.kind, start, params, dialect);
    sql.push_str(&format!(" WHEN {when_sql} THEN {then_sql}"));
  }
  if let Some(value) = otherwise {
    let else_sql = render_scalar(&value.kind, start, params, dialect);
    sql.push_str(&format!(" ELSE {else_sql}"));
  }
  sql.push_str(" END");
  sql
}
