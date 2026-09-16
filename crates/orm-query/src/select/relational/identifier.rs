//! Quoting of qualified SQL identifiers for relational statements.

/// Appends `"qualifier"."column"` to the SQL buffer.
///
/// Both parts are quoted, and an embedded `"` is doubled — the standard escape
/// for a delimited identifier in SQLite and Postgres alike — so a name carrying
/// a quote cannot close the quoting context early.
pub(super) fn push_qualified(sql: &mut String, qualifier: &str, column: &str) {
  push_quoted(sql, qualifier);
  sql.push('.');
  push_quoted(sql, column);
}

fn push_quoted(sql: &mut String, identifier: &str) {
  sql.push('"');
  for ch in identifier.chars() {
    if ch == '"' {
      sql.push('"');
    }
    sql.push(ch);
  }
  sql.push('"');
}
