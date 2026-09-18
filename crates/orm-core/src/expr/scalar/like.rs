//! Turning user text into a `LIKE` pattern that matches it literally.

/// Prefix every `%`, `_` and `escape` in `text` with `escape`, returning a
/// pattern that matches `text` literally under `LIKE … ESCAPE escape`.
///
/// Wildcards typed by a user are the reason this exists: without it,
/// searching for `100%` matches `100 percent` too. Wrap the result in `%…%`
/// yourself when you want a contains-search.
///
/// ```
/// use toolu_orm_core::expr::like_pattern_literal;
///
/// assert_eq!(like_pattern_literal("100%", '\\'), r"100\%");
/// assert_eq!(like_pattern_literal("a_b", '\\'), r"a\_b");
/// assert_eq!(like_pattern_literal(r"back\slash", '\\'), r"back\\slash");
/// assert_eq!(like_pattern_literal("", '\\'), "");
/// ```
#[must_use]
pub fn like_pattern_literal(text: &str, escape: char) -> String {
  let mut pattern = String::with_capacity(text.len() + 8);
  for ch in text.chars() {
    if ch == '%' || ch == '_' || ch == escape {
      pattern.push(escape);
    }
    pattern.push(ch);
  }
  pattern
}
