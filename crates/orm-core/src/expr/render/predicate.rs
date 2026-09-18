//! Dialect-aware SQL rendering for [`ExprKind`] trees.

use crate::dialect::Dialect;
use crate::expr::binding::ListSource;
use crate::expr::types::ExprKind;
use crate::expr::{BoundParams, Scalar};
use crate::value::Value;

use super::raw_params::number_raw_params;
use super::scalar::render_scalar;
use super::subquery::{render_exists, render_in_subquery};

/// Renders a predicate node, appending the values it binds to `params`.
///
/// Position lives in `params`: every binding arm takes
/// [`BoundParams::next_index`] at the moment it writes, so a node numbers from
/// whatever its siblings already emitted however deeply it is nested and no
/// call site adds an offset. See [`render_scalar`] for the same contract on
/// the value half, and [`BoundParams::nested`] for the one place that re-bases
/// the count.
///
/// A shared binding is the one arm that may not push: `params` hands back the
/// index it already recorded for that handle, leaving the live length — and so
/// every later arm's numbering — untouched.
pub(crate) fn render_expr(kind: &ExprKind, params: &mut BoundParams, dialect: Dialect) -> String {
  match kind {
    ExprKind::Comparison { column, op, value } => {
      let idx = params.bind(value);
      format!("{column} {op} {}", dialect.param(idx))
    },
    ExprKind::InList {
      column,
      values,
      negated,
    } => render_in_list(column, values, *negated, params, dialect),
    ExprKind::IsNull { column, negated } => {
      if *negated {
        format!("{column} IS NOT NULL")
      } else {
        format!("{column} IS NULL")
      }
    },
    ExprKind::Between { column, low, high } => {
      let low_idx = params.push(low.clone());
      let high_idx = params.push(high.clone());
      format!(
        "{column} BETWEEN {} AND {}",
        dialect.param(low_idx),
        dialect.param(high_idx)
      )
    },
    ExprKind::Match { target, pattern } => {
      let idx = params.push(pattern.clone());
      format!("{target} MATCH {}", dialect.param(idx))
    },
    ExprKind::TsMatch {
      document,
      query_fn,
      config,
      pattern,
    } => {
      let idx = params.push(pattern.clone());
      format!("{document} @@ {query_fn}({config}, {})", dialect.param(idx))
    },
    ExprKind::Compare { left, op, right } => {
      let left_sql = render_scalar(&left.kind, params, dialect);
      let right_sql = render_scalar(&right.kind, params, dialect);
      format!("{left_sql} {op} {right_sql}")
    },
    ExprKind::Like {
      left,
      pattern,
      escape,
    } => render_like(left, pattern, *escape, params, dialect),
    ExprKind::Exists { query, negated } => render_exists(query.as_ref(), *negated, params, dialect),
    ExprKind::InSubquery {
      left,
      query,
      negated,
    } => render_in_subquery(left, query.as_ref(), *negated, params, dialect),
    ExprKind::And(left, right) => {
      let left_sql = render_expr(&left.kind, params, dialect);
      let right_sql = render_expr(&right.kind, params, dialect);
      format!("({left_sql} AND {right_sql})")
    },
    ExprKind::Or(left, right) => {
      let left_sql = render_expr(&left.kind, params, dialect);
      let right_sql = render_expr(&right.kind, params, dialect);
      format!("({left_sql} OR {right_sql})")
    },
    ExprKind::Raw {
      sql,
      params: raw_params,
    } => {
      let base = params.next_index();
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
  params: &mut BoundParams,
  dialect: Dialect,
) -> String {
  let left_sql = render_scalar(&left.kind, params, dialect);
  let pattern_sql = render_scalar(&pattern.kind, params, dialect);
  if let Some(character) = escape {
    let idx = params.push(Value::Text(character.to_string()));
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
///
/// A shared list binds its values once; every later occurrence renders the
/// same contiguous run, which is what `bind_list` hands back.
fn render_in_list(
  column: &str,
  values: &ListSource,
  negated: bool,
  params: &mut BoundParams,
  dialect: Dialect,
) -> String {
  let indices = params.bind_list(values);
  if indices.is_empty() {
    return if negated { "1 = 1" } else { "1 = 0" }.to_owned();
  }
  let placeholders: Vec<String> = indices.map(|index| dialect.param(index)).collect();
  let keyword = if negated { "NOT IN" } else { "IN" };
  format!("{column} {keyword} ({})", placeholders.join(", "))
}
