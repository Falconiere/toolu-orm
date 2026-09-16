//! Quoting of qualified SQL identifiers for relational statements.

/// Appends `"qualifier"."column"` to the SQL buffer.
pub(super) fn push_qualified(sql: &mut String, qualifier: &str, column: &str) {
  sql.push('"');
  sql.push_str(qualifier);
  sql.push_str("\".\"");
  sql.push_str(column);
  sql.push('"');
}
