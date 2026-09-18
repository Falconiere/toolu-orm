//! Double-quoting of the identifiers this module renders.

/// A double-quoted identifier; an embedded double quote doubles.
///
/// Doubling is the only escape a delimited identifier has, in SQLite and
/// Postgres alike, so once every interior quote is doubled the sole unpaired
/// quotes are the delimiters. `toolu_orm_core`'s alias module keeps the same
/// escape for the identifiers *it* renders
/// (`crates/orm-core/src/alias/quoting.rs`), private to that module.
///
/// Every name reaching this function is a `&'static str` written in Rust
/// source — `Column::new("memories", "id")`, `InsertBuilder::new("memories")`
/// — so for a legal identifier the result is byte-identical to a bare wrap.
/// Doubling costs nothing and removes the question.
pub(super) fn quote_ident(ident: &str) -> String {
  format!("\"{}\"", ident.replace('"', "\"\""))
}
