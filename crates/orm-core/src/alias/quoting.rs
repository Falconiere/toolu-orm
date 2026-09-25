//! Double-quoting of a SQL identifier, with the embedded-quote escape.

/// A double-quoted identifier; an embedded double quote doubles.
///
/// Doubling is the only escape a delimited identifier has in SQLite, Postgres,
/// and Lance — there is no backslash escape to honour, and no other
/// character is special inside the quotes. Once every interior quote is
/// doubled the sole unpaired quotes are the delimiters, so a backslash, `;`, a
/// newline or `--` inside a name are ordinary characters of it rather than a
/// way out of the identifier.
///
/// [`Dialect::quote_ident`](crate::dialect::Dialect::quote_ident) delegates to
/// this function for every query dialect.
pub fn quote_ident(ident: &str) -> String {
  format!("\"{}\"", ident.replace('"', "\"\""))
}
