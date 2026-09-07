//! Postgres lateral join SQL generation for relational SELECT queries.

use super::config::RelationalSelectBuilder;

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
        Self::push_column_list(&mut sql, &target_alias, &rel.target_columns);
        sql.push_str(")), '[]'::json) AS \"data\"");
      } else {
        sql.push_str("json_build_array(");
        Self::push_column_list(&mut sql, &target_alias, &rel.target_columns);
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
