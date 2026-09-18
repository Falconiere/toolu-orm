//! Dialect-aware SQL rendering for [`ScalarKind`] trees.

use crate::dialect::Dialect;
use crate::expr::scalar::{AggregateArg, Scalar, ScalarKind};
use crate::expr::{BoundParams, Expr};

use super::predicate::render_expr;
use super::raw_params::number_raw_params;
use super::subquery::render_scalar_subquery;

/// Renders a scalar node, appending the values it binds to `params`.
///
/// Position lives in `params`: each binding arm takes
/// [`BoundParams::next_index`] at the moment it pushes, so a node numbers from
/// whatever its siblings already emitted however deeply it is nested. Recursive
/// calls therefore pass nothing but the buffer — there is no base to add, and
/// so nothing to double-count. `render_expr` follows the same contract, and
/// [`BoundParams::nested`] is the one place that re-bases the count for a
/// fragment spliced after other clauses.
pub(crate) fn render_scalar(
  kind: &ScalarKind,
  params: &mut BoundParams,
  dialect: Dialect,
) -> String {
  match kind {
    ScalarKind::Raw {
      sql,
      params: raw_params,
    } => {
      let base = params.next_index();
      params.extend(raw_params.iter().cloned());
      number_raw_params(sql, base, dialect)
    },
    ScalarKind::Bind(value) => dialect.param(params.bind(value)),
    ScalarKind::Func { name, args } => {
      let rendered: Vec<String> = args
        .iter()
        .map(|arg| render_scalar(&arg.kind, params, dialect))
        .collect();
      format!("{name}({})", rendered.join(", "))
    },
    ScalarKind::Arith { left, op, right } => {
      let left_sql = render_scalar(&left.kind, params, dialect);
      let right_sql = render_scalar(&right.kind, params, dialect);
      format!("({left_sql} {op} {right_sql})")
    },
    ScalarKind::Case {
      branches,
      otherwise,
    } => render_case(branches, otherwise.as_deref(), params, dialect),
    ScalarKind::Aggregate { func, arg } => {
      format!("{func}({})", render_aggregate_arg(arg, params, dialect))
    },
    ScalarKind::Subquery(query) => render_scalar_subquery(query.as_ref(), params, dialect),
  }
}

/// `*`, the argument, or `DISTINCT ` before it.
///
/// The argument renders through `render_scalar` like any other child, so an
/// aggregate over a bound value numbers from whatever its siblings emitted.
fn render_aggregate_arg(arg: &AggregateArg, params: &mut BoundParams, dialect: Dialect) -> String {
  match arg {
    AggregateArg::Star => "*".to_owned(),
    AggregateArg::All(inner) => render_scalar(&inner.kind, params, dialect),
    AggregateArg::Distinct(inner) => {
      let rendered = render_scalar(&inner.kind, params, dialect);
      format!("DISTINCT {rendered}")
    },
  }
}

/// `CASE WHEN … THEN … [ELSE …] END`, each branch numbered after the last.
fn render_case(
  branches: &[(Expr, Scalar)],
  otherwise: Option<&Scalar>,
  params: &mut BoundParams,
  dialect: Dialect,
) -> String {
  let mut sql = String::from("CASE");
  for (predicate, then) in branches {
    let when_sql = render_expr(&predicate.kind, params, dialect);
    let then_sql = render_scalar(&then.kind, params, dialect);
    sql.push_str(&format!(" WHEN {when_sql} THEN {then_sql}"));
  }
  if let Some(value) = otherwise {
    let else_sql = render_scalar(&value.kind, params, dialect);
    sql.push_str(&format!(" ELSE {else_sql}"));
  }
  sql.push_str(" END");
  sql
}
