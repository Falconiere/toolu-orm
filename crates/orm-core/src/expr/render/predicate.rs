//! Dialect-aware SQL rendering for [`ExprKind`] trees.

use crate::dialect::Dialect;
use crate::expr::types::ExprKind;
use crate::expr::Scalar;
use crate::value::Value;

use super::raw_params::number_raw_params;
use super::scalar::render_scalar;
use super::subquery::{render_exists, render_in_subquery};

/// Renders a predicate node, appending the values it binds to `params`.
///
/// `start` is the index of the statement's first parameter and stays constant
/// down the tree: each binding arm adds the live `params.len()` when it
/// pushes, so children take `start` unchanged. See [`render_scalar`] for the
/// same contract on the value half.
pub(crate) fn render_expr(
  kind: &ExprKind,
  start: usize,
  params: &mut Vec<Value>,
  dialect: Dialect,
) -> String {
  match kind {
    ExprKind::Comparison { column, op, value } => {
      let idx = start + params.len();
      params.push(value.clone());
      format!("{column} {op} {}", dialect.param(idx))
    },
    ExprKind::InList {
      column,
      values,
      negated,
    } => render_in_list(column, values, *negated, start, params, dialect),
    ExprKind::IsNull { column, negated } => {
      if *negated {
        format!("{column} IS NOT NULL")
      } else {
        format!("{column} IS NULL")
      }
    },
    ExprKind::Between { column, low, high } => {
      let low_idx = start + params.len();
      params.push(low.clone());
      let high_idx = start + params.len();
      params.push(high.clone());
      format!(
        "{column} BETWEEN {} AND {}",
        dialect.param(low_idx),
        dialect.param(high_idx)
      )
    },
    ExprKind::Match { target, pattern } => {
      let idx = start + params.len();
      params.push(pattern.clone());
      format!("{target} MATCH {}", dialect.param(idx))
    },
    ExprKind::TsMatch {
      document,
      query_fn,
      config,
      pattern,
    } => {
      let idx = start + params.len();
      params.push(pattern.clone());
      format!("{document} @@ {query_fn}({config}, {})", dialect.param(idx))
    },
    ExprKind::Compare { left, op, right } => {
      let left_sql = render_scalar(&left.kind, start, params, dialect);
      let right_sql = render_scalar(&right.kind, start, params, dialect);
      format!("{left_sql} {op} {right_sql}")
    },
    ExprKind::Like {
      left,
      pattern,
      escape,
    } => render_like(left, pattern, *escape, start, params, dialect),
    ExprKind::Exists { query, negated } => {
      render_exists(query.as_ref(), *negated, start, params, dialect)
    },
    ExprKind::InSubquery {
      left,
      query,
      negated,
    } => render_in_subquery(left, query.as_ref(), *negated, start, params, dialect),
    ExprKind::And(left, right) => {
      let left_sql = render_expr(&left.kind, start, params, dialect);
      let right_sql = render_expr(&right.kind, start, params, dialect);
      format!("({left_sql} AND {right_sql})")
    },
    ExprKind::Or(left, right) => {
      let left_sql = render_expr(&left.kind, start, params, dialect);
      let right_sql = render_expr(&right.kind, start, params, dialect);
      format!("({left_sql} OR {right_sql})")
    },
    ExprKind::Raw {
      sql,
      params: raw_params,
    } => {
      let base = start + params.len();
      params.extend(raw_params.iter().cloned());
      number_raw_params(sql, base, dialect)
    },
  }
}

/// `<left> LIKE <pattern> [ESCAPE ?N]`.
///
/// The escape character is bound like any other value — both SQLite and
/// Postgres accept a parameter in this position — so it travels through the
/// same numbering as the rest of the tree and no character is interpolated
/// into the SQL.
fn render_like(
  left: &Scalar,
  pattern: &Scalar,
  escape: Option<char>,
  start: usize,
  params: &mut Vec<Value>,
  dialect: Dialect,
) -> String {
  let left_sql = render_scalar(&left.kind, start, params, dialect);
  let pattern_sql = render_scalar(&pattern.kind, start, params, dialect);
  if let Some(character) = escape {
    let idx = start + params.len();
    params.push(Value::Text(character.to_string()));
    format!(
      "{left_sql} LIKE {pattern_sql} ESCAPE {}",
      dialect.param(idx)
    )
  } else {
    format!("{left_sql} LIKE {pattern_sql}")
  }
}

/// `col IN (?N, ...)` / `col NOT IN (...)`. An empty list renders a constant
/// (`1 = 0` / `1 = 1`, like drizzle): `IN ()` is a syntax error on Postgres,
/// and SQLite only tolerates it as false.
fn render_in_list(
  column: &str,
  values: &[Value],
  negated: bool,
  start: usize,
  params: &mut Vec<Value>,
  dialect: Dialect,
) -> String {
  if values.is_empty() {
    return if negated { "1 = 1" } else { "1 = 0" }.to_owned();
  }
  let base = start + params.len();
  let placeholders: Vec<String> = values
    .iter()
    .enumerate()
    .map(|(i, _)| dialect.param(base + i))
    .collect();
  params.extend(values.iter().cloned());
  let keyword = if negated { "NOT IN" } else { "IN" };
  format!("{column} {keyword} ({})", placeholders.join(", "))
}
