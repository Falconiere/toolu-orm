//! Placeholder renumbering shared by every raw-SQL node.

use crate::dialect::Dialect;

/// Replace bare `?` (not already `?N`) with sequential placeholders starting at `start`.
pub(super) fn number_raw_params(sql: &str, start: usize, dialect: Dialect) -> String {
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
