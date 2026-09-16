//! RelationalSelectBuilder and RelationConfig for eager-loaded relation queries.

use super::identifier::push_qualified;
use super::relation_column::RelationColumn;

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
  pub target_columns: Vec<RelationColumn>,
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
    target_columns: Vec<RelationColumn>,
    is_many: bool,
  ) -> Self {
    Self {
      field_name: field_name.to_owned(),
      target_table: target_table.to_owned(),
      local_key: local_key.to_owned(),
      foreign_key: foreign_key.to_owned(),
      target_columns,
      is_many,
      nested: Vec::new(),
    }
  }
}

/// `&["id", "title"]` → plain, non-binary relation columns.
fn plain_columns(names: &[&str]) -> Vec<RelationColumn> {
  names.iter().map(|c| RelationColumn::new(c)).collect()
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
    self,
    field_name: &str,
    target_table: &str,
    local_key: &str,
    foreign_key: &str,
    target_columns: &[&str],
  ) -> Self {
    self.with_many_columns(
      field_name,
      target_table,
      local_key,
      foreign_key,
      &plain_columns(target_columns),
    )
  }

  /// Add a has-one / belongs-to relation.
  pub fn with_one(
    self,
    field_name: &str,
    target_table: &str,
    local_key: &str,
    foreign_key: &str,
    target_columns: &[&str],
  ) -> Self {
    self.with_one_columns(
      field_name,
      target_table,
      local_key,
      foreign_key,
      &plain_columns(target_columns),
    )
  }

  /// Add a has-many relation whose columns declare their transport, so a
  /// [`RelationColumn::binary`] column survives the JSON round trip.
  pub fn with_many_columns(
    mut self,
    field_name: &str,
    target_table: &str,
    local_key: &str,
    foreign_key: &str,
    target_columns: &[RelationColumn],
  ) -> Self {
    self.relations.push(RelationConfig::new(
      field_name,
      target_table,
      local_key,
      foreign_key,
      target_columns.to_vec(),
      true,
    ));
    self
  }

  /// Add a has-one / belongs-to relation whose columns declare their transport.
  pub fn with_one_columns(
    mut self,
    field_name: &str,
    target_table: &str,
    local_key: &str,
    foreign_key: &str,
    target_columns: &[RelationColumn],
  ) -> Self {
    self.relations.push(RelationConfig::new(
      field_name,
      target_table,
      local_key,
      foreign_key,
      target_columns.to_vec(),
      false,
    ));
    self
  }

  /// The relation declared under `field_name`, if any.
  pub(super) fn relation(&self, field_name: &str) -> Option<&RelationConfig> {
    self.relations.iter().find(|r| r.field_name == field_name)
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
