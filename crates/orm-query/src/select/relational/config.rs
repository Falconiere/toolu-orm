//! RelationalSelectBuilder and RelationConfig for eager-loaded relation queries.

/// Configuration for a single relation to be loaded.
#[derive(Debug, Clone)]
pub struct RelationConfig {
  /// The field name in the result struct (e.g. "posts").
  pub field_name: String,
  /// The target table to join against.
  pub target_table: String,
  /// The column on the source table (e.g., "id").
  pub local_key: String,
  /// The column on the target table (e.g., "author_id").
  pub foreign_key: String,
  /// The columns to select from the target table.
  pub target_columns: Vec<String>,
  /// `true` for has-many, `false` for has-one / belongs-to.
  pub is_many: bool,
  /// Nested relations on the target table (reserved).
  pub nested: Vec<RelationConfig>,
}

impl RelationConfig {
  fn new(
    field_name: &str,
    target_table: &str,
    local_key: &str,
    foreign_key: &str,
    target_columns: &[&str],
    is_many: bool,
  ) -> Self {
    Self {
      field_name: field_name.to_owned(),
      target_table: target_table.to_owned(),
      local_key: local_key.to_owned(),
      foreign_key: foreign_key.to_owned(),
      target_columns: target_columns.iter().map(|c| (*c).to_owned()).collect(),
      is_many,
      nested: Vec::new(),
    }
  }
}

/// Builder for relational SELECT queries.
pub struct RelationalSelectBuilder {
  pub(super) source_table: String,
  pub(super) source_columns: Vec<String>,
  pub(super) relations: Vec<RelationConfig>,
}

impl RelationalSelectBuilder {
  /// New builder for the given source table and scalar columns.
  pub fn new(source_table: &str, columns: &[&str]) -> Self {
    Self {
      source_table: source_table.to_owned(),
      source_columns: columns.iter().map(|c| (*c).to_owned()).collect(),
      relations: Vec::new(),
    }
  }

  /// Add a has-many relation.
  pub fn with_many(
    mut self,
    field_name: &str,
    target_table: &str,
    local_key: &str,
    foreign_key: &str,
    target_columns: &[&str],
  ) -> Self {
    self.relations.push(RelationConfig::new(
      field_name,
      target_table,
      local_key,
      foreign_key,
      target_columns,
      true,
    ));
    self
  }

  /// Add a has-one / belongs-to relation.
  pub fn with_one(
    mut self,
    field_name: &str,
    target_table: &str,
    local_key: &str,
    foreign_key: &str,
    target_columns: &[&str],
  ) -> Self {
    self.relations.push(RelationConfig::new(
      field_name,
      target_table,
      local_key,
      foreign_key,
      target_columns,
      false,
    ));
    self
  }

  /// Relation configurations in declaration order.
  pub fn relation_configs(&self) -> &[RelationConfig] {
    &self.relations
  }

  /// Source table name.
  pub fn source_table(&self) -> &str {
    &self.source_table
  }

  /// Scalar column names on the source table.
  pub fn source_columns(&self) -> &[String] {
    &self.source_columns
  }

  // ── Shared SQL helpers (used by both postgres_sql and sqlite_sql) ──────────

  /// Appends `"table"."col1", "table"."col2", ...` for source columns.
  pub(super) fn push_source_select(&self, sql: &mut String) {
    for (i, col) in self.source_columns.iter().enumerate() {
      if i > 0 {
        sql.push_str(", ");
      }
      push_qualified(sql, &self.source_table, col);
    }
  }

  /// Appends `"qualifier"."col1", "qualifier"."col2", ...` for a column list.
  pub(super) fn push_column_list(sql: &mut String, qualifier: &str, columns: &[String]) {
    for (i, col) in columns.iter().enumerate() {
      if i > 0 {
        sql.push_str(", ");
      }
      push_qualified(sql, qualifier, col);
    }
  }

  /// Appends `WHERE "target"."fk" = "source"."lk"`.
  pub(super) fn push_join_where(
    &self,
    sql: &mut String,
    target_qualifier: &str,
    rel: &RelationConfig,
  ) {
    sql.push_str(" WHERE ");
    push_qualified(sql, target_qualifier, &rel.foreign_key);
    sql.push_str(" = ");
    push_qualified(sql, &self.source_table, &rel.local_key);
    if !rel.is_many {
      sql.push_str(" LIMIT 1");
    }
  }
}

/// Appends `"qualifier"."column"` to the SQL buffer.
fn push_qualified(sql: &mut String, qualifier: &str, column: &str) {
  sql.push('"');
  sql.push_str(qualifier);
  sql.push_str("\".\"");
  sql.push_str(column);
  sql.push('"');
}
