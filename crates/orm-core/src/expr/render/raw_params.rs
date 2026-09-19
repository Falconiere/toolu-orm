//! Placeholder renumbering shared by every raw-SQL node.

use std::iter::Peekable;
use std::str::Chars;

use crate::dialect::Dialect;

/// Renders a fragment's placeholders in its own frame, where `start` is the
/// index its first value takes. Every number is shifted by `start - 1`, so a
/// placeholder always names the value it was written for (issue #131).
///
/// - `?N` (`N >= 1`) addresses the fragment's own `N`-th value; the same `N`
///   twice is one parameter referenced twice.
/// - A bare `?` takes one more than the largest number assigned so far, which
///   is SQLite's own rule for anonymous parameters — unless no `usize` can
///   hold that successor, when it too stays verbatim.
/// - `?0`, and a digit run that no `usize` can hold or shift, are not
///   placeholders: emitted verbatim, so the engine refuses to prepare rather
///   than answer with the wrong value. An over-count shifts like any other
///   number — see [`crate::expr::Expr::raw`] for that hazard.
///
/// `start` is 1-based and both call sites pass [`BoundParams::next_index`],
/// which is `len() + 1` and so never 0; the saturating subtraction below is the
/// same guard [`BoundParams::nested`] uses, and renders a 0 as base 1 rather
/// than wrapping.
///
/// [`BoundParams::next_index`]: crate::expr::BoundParams::next_index
/// [`BoundParams::nested`]: crate::expr::BoundParams::nested
pub(super) fn number_raw_params(sql: &str, start: usize, dialect: Dialect) -> String {
  let mut result = String::with_capacity(sql.len() + 8);
  let offset = start.saturating_sub(1);
  let mut highest = 0;
  let mut chars = sql.chars().peekable();

  while let Some(ch) = chars.next() {
    if ch != '?' {
      result.push(ch);
      continue;
    }

    let digits = take_digits(&mut chars);
    if let Some((number, index)) = placeholder(&digits, highest, offset) {
      result.push_str(&dialect.param(index));
      highest = highest.max(number);
    } else {
      result.push('?');
      result.push_str(&digits);
    }
  }

  result
}

/// The digit run directly after a `?`, empty when the placeholder is bare.
fn take_digits(chars: &mut Peekable<Chars<'_>>) -> String {
  let mut digits = String::new();
  while let Some(digit) = chars.next_if(char::is_ascii_digit) {
    digits.push(digit);
  }
  digits
}

/// Which of the fragment's own values a token addresses and the index it
/// renders at, or `None` when the token is not a placeholder this renderer
/// owns: `?0`, or digits no `usize` can hold or shift by `offset`.
fn placeholder(digits: &str, highest: usize, offset: usize) -> Option<(usize, usize)> {
  let number = if digits.is_empty() {
    highest.checked_add(1)?
  } else {
    match digits.parse::<usize>() {
      Ok(parsed) if parsed >= 1 => parsed,
      _ => return None,
    }
  };
  Some((number, number.checked_add(offset)?))
}
