//! SQLite correlated subquery SQL generation for relational SELECT queries.

use super::config::RelationalSelectBuilder;

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
        Self::push_column_list(&mut sql, &rel.target_table, &rel.target_columns);
        sql.push_str(")), json_array())");
      } else {
        sql.push_str("json_array(");
        Self::push_column_list(&mut sql, &rel.target_table, &rel.target_columns);
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
