//! SQLite correlated subquery SQL generation for relational SELECT queries.

use super::config::RelationalSelectBuilder;
use super::identifier::push_qualified;
use super::relation_column::RelationColumn;

/// Appends the projected columns of one relation.
///
/// A binary column becomes `CASE WHEN "t"."c" IS NULL THEN NULL ELSE hex("t"."c") END`:
/// `json_array` rejects a `BLOB` argument, and a bare `hex()` would map SQL NULL
/// to `''`, collapsing it into the empty blob.
fn push_target_columns(sql: &mut String, qualifier: &str, columns: &[RelationColumn]) {
  for (i, col) in columns.iter().enumerate() {
    if i > 0 {
      sql.push_str(", ");
    }
    if col.is_binary() {
      sql.push_str("CASE WHEN ");
      push_qualified(sql, qualifier, col.name());
      sql.push_str(" IS NULL THEN NULL ELSE hex(");
      push_qualified(sql, qualifier, col.name());
      sql.push_str(") END");
    } else {
      push_qualified(sql, qualifier, col.name());
    }
  }
}

impl RelationalSelectBuilder {
  /// SQLite: correlated subqueries + `json_group_array` / `json_array`.
  pub fn to_sql_sqlite(&self) -> String {
    let mut sql = String::new();
    sql.push_str("SELECT ");
    self.push_source_select(&mut sql);

    for rel in &self.relations {
      sql.push_str(", (SELECT ");
      if rel.is_many {
        sql.push_str("coalesce(json_group_array(json_array(");
        push_target_columns(&mut sql, &rel.target_table, &rel.target_columns);
        sql.push_str(")), json_array())");
      } else {
        sql.push_str("json_array(");
        push_target_columns(&mut sql, &rel.target_table, &rel.target_columns);
        sql.push(')');
      }

      sql.push_str(" FROM \"");
      sql.push_str(&rel.target_table);
      sql.push('"');
      self.push_join_where(&mut sql, &rel.target_table, rel);

      sql.push_str(") AS \"");
      sql.push_str(&rel.field_name);
      sql.push('"');
    }

    sql.push_str(" FROM \"");
    sql.push_str(&self.source_table);
    sql.push('"');

    sql
  }

  /// SQL for the active driver feature (`postgres` vs default sqlite/libsql).
  pub fn to_sql(&self) -> String {
    #[cfg(feature = "postgres")]
    {
      self.to_sql_postgres()
    }
    #[cfg(not(feature = "postgres"))]
    {
      self.to_sql_sqlite()
    }
  }
}
