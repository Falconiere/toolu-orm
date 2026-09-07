//! Dialect-aware SQL rendering for [`ExprKind`] trees.

use crate::dialect::Dialect;
use crate::value::Value;

use super::types::ExprKind;

pub(super) fn render_expr(
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
      params.extend(raw_params.iter().cloned());
      number_raw_params(sql, start, dialect)
    },
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

/// Replace bare `?` (not already `?N`) with sequential placeholders starting at `start`.
fn number_raw_params(sql: &str, start: usize, dialect: Dialect) -> String {
  let mut result = String::with_capacity(sql.len() + 8);
  let mut counter = start;
  let mut chars = sql.chars().peekable();

  while let Some(ch) = chars.next() {
    if ch != '?' {
      result.push(ch);
      continue;
    }
    // ch == '?': check if next char is a digit (already numbered)
    if chars.peek().is_some_and(|c| c.is_ascii_digit()) {
      let mut num_str = String::new();
      while chars.peek().is_some_and(|c| c.is_ascii_digit()) {
        if let Some(d) = chars.next() {
          num_str.push(d);
        }
      }
      let idx: usize = num_str.parse().unwrap_or(counter);
      result.push_str(&dialect.param(idx));
    } else {
      result.push_str(&dialect.param(counter));
      counter += 1;
    }
  }

  result
}
