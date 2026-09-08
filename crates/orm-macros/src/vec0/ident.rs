//! The identifier rule `vec0` enforces — duplicated from
//! `toolu_orm_core::vec0::is_vec0_ident`.
//!
//! Kept local so the proc-macro crate does not link `orm-core` (which, under
//! `libsql`+`rusqlite`, would pull two SQLite C archives into the macro dylib
//! and fail the linker). Must stay identical to the core function; the
//! `vec0_macro_test` suite builds the same table both ways and compares.

/// True when `vec0`'s scanner reads `name` as exactly one identifier.
#[must_use]
pub(super) fn is_vec0_ident(name: &str) -> bool {
  let mut chars = name.chars();
  if !chars
    .next()
    .is_some_and(|first| first.is_ascii_alphabetic())
  {
    return false;
  }
  chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}
