//! The identifier rule `vec0` enforces inside its own constructor.
//!
//! The `#[vec0_table]` proc macro keeps an identical copy under
//! `orm-macros::vec0::ident` so the macro dylib never links `orm-core` (which
//! would pull two SQLite C archives under `libsql`+`rusqlite`). Keep them in
//! lockstep.

/// True when `vec0`'s scanner reads `name` as exactly one identifier.
///
/// `sqlite-vec` parses the text between the module parentheses itself, with a
/// scanner that knows only `[A-Za-z]`-led words of `[A-Za-z0-9_]`, digits and
/// the punctuation `+ [ ] = ( ) ,`. A double quote is a scan error, so unlike
/// FTS5 there is no quoting to fall back on: a name outside this rule cannot
/// be rendered at all, and [`super::Vec0Table::build`] refuses it.
#[must_use]
pub fn is_vec0_ident(name: &str) -> bool {
  let mut chars = name.chars();
  if !chars
    .next()
    .is_some_and(|first| first.is_ascii_alphabetic())
  {
    return false;
  }
  chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}
