//! Postgres lateral join SQL generation for relational SELECT queries.

use super::config::{push_qualified, RelationalSelectBuilder};
use super::relation_column::RelationColumn;

/// Appends the projected columns of one relation.
///
/// A binary column becomes `encode("t"."c", 'hex')`: `json_build_array` would
/// otherwise render `bytea` through its text output as `"\\x0102"`. `encode` is
/// strict, so SQL NULL stays NULL.
fn push_target_columns(sql: &mut String, qualifier: &str, columns: &[RelationColumn]) {
  for (i, col) in columns.iter().enumerate() {
    if i > 0 {
      sql.push_str(", ");
    }
    if col.is_binary() {
      sql.push_str("encode(");
      push_qualified(sql, qualifier, col.name());
      sql.push_str(", 'hex')");
    } else {
      push_qualified(sql, qualifier, col.name());
    }
  }
}

impl RelationalSelectBuilder {
  /// Postgres: `LEFT JOIN LATERAL` + `json_agg` / `json_build_array`.
  pub fn to_sql_postgres(&self) -> String {
    let mut sql = String::new();
    sql.push_str("SELECT ");
    self.push_source_select(&mut sql);

    for rel in &self.relations {
      let alias = format!("{}_{}", self.source_table, rel.field_name);
      sql.push_str(", \"");
      sql.push_str(&alias);
      sql.push_str("\".\"data\" AS \"");
      sql.push_str(&rel.field_name);
      sql.push('"');
    }

    sql.push_str(" FROM \"");
    sql.push_str(&self.source_table);
    sql.push('"');

    for rel in &self.relations {
      let alias = format!("{}_{}", self.source_table, rel.field_name);
      let target_alias = format!("{}_sub", rel.field_name);
      sql.push_str(" LEFT JOIN LATERAL (SELECT ");

      if rel.is_many {
        sql.push_str("coalesce(json_agg(json_build_array(");
        push_target_columns(&mut sql, &target_alias, &rel.target_columns);
        sql.push_str(")), '[]'::json) AS \"data\"");
      } else {
        sql.push_str("json_build_array(");
        push_target_columns(&mut sql, &target_alias, &rel.target_columns);
        sql.push_str(") AS \"data\"");
      }

      sql.push_str(" FROM \"");
      sql.push_str(&rel.target_table);
      sql.push_str("\" \"");
      sql.push_str(&target_alias);
      sql.push('"');
      self.push_join_where(&mut sql, &target_alias, rel);

      sql.push_str(") \"");
      sql.push_str(&alias);
      sql.push_str("\" ON true");
    }

    sql
  }
}
