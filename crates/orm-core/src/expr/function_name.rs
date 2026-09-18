//! The one rule for a SQL function name that this crate is allowed to render.
//!
//! A function name is SQL *syntax*, not data: it sits outside quotes, so there
//! is no escape that would make an arbitrary string safe there. It is
//! therefore validated rather than escaped — anything outside
//! `[A-Za-z_][A-Za-z0-9_]*` is a caller mistake and never reaches a statement.
//!
//! Both call sites share this predicate so the scalar form
//! ([`Scalar::func`](crate::expr::Scalar::func)) and the table-valued form
//! ([`TableRef::function`](crate::alias::TableRef::function)) cannot drift
//! apart; each maps a rejection to its own error variant.

/// Whether `name` may be rendered unquoted as a SQL function name.
///
/// ASCII letters, digits and underscores only, and never a leading digit. An
/// empty name is rejected: it would render `(…)`, which is not a call.
pub(crate) fn is_valid_function_name(name: &str) -> bool {
  let mut chars = name.chars();
  let Some(first) = chars.next() else {
    return false;
  };
  if !(first.is_ascii_alphabetic() || first == '_') {
    return false;
  }
  chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}
