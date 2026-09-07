//! Core types for table relations used by relational query building.
//!
//! # Public API
//!
//! - [`RelationKind`], [`JoinColumn`], [`ThroughDef`], [`RelationDef`]
//! - [`RelationRegistry`], helpers [`one`], [`many`], [`many_through`]

use std::collections::BTreeMap;

/// The kind of relation between two tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationKind {
  /// Exactly zero or one related row (foreign key on the source table).
  One,
  /// Zero or more related rows (foreign key on the target table).
  Many,
}

/// A single column pair in a join condition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinColumn {
  /// Column name on the source (local) table.
  pub local: String,
  /// Column name on the target (foreign) table.
  pub foreign: String,
}

/// Configuration for many-to-many relations via a junction table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThroughDef {
  /// The junction (pivot) table name.
  pub junction_table: String,
  /// Column in the junction table that references the source table.
  pub local_column: String,
  /// Column in the junction table that references the target table.
  pub foreign_column: String,
}

/// A complete relation definition between two tables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationDef {
  /// The field name used in result structs (e.g. "posts", "author").
  pub field_name: String,
  /// Whether this is a one-to-one/many-to-one or one-to-many relation.
  pub kind: RelationKind,
  /// The target table name.
  pub target_table: String,
  /// The join column pairs (local column -> foreign column).
  pub join_columns: Vec<JoinColumn>,
  /// For many-to-many: the junction table configuration.
  pub through: Option<ThroughDef>,
}

/// Create a `One` (belongs-to / has-one) relation.
///
/// `columns` is a slice of `(local_column, foreign_column)` pairs.
pub fn one(field_name: &str, target_table: &str, columns: &[(&str, &str)]) -> RelationDef {
  RelationDef {
    field_name: field_name.to_owned(),
    kind: RelationKind::One,
    target_table: target_table.to_owned(),
    join_columns: columns
      .iter()
      .map(|(local, foreign)| JoinColumn {
        local: (*local).to_owned(),
        foreign: (*foreign).to_owned(),
      })
      .collect(),
    through: None,
  }
}

/// Create a `Many` (has-many) relation.
///
/// `columns` is a slice of `(local_column, foreign_column)` pairs.
pub fn many(field_name: &str, target_table: &str, columns: &[(&str, &str)]) -> RelationDef {
  RelationDef {
    field_name: field_name.to_owned(),
    kind: RelationKind::Many,
    target_table: target_table.to_owned(),
    join_columns: columns
      .iter()
      .map(|(local, foreign)| JoinColumn {
        local: (*local).to_owned(),
        foreign: (*foreign).to_owned(),
      })
      .collect(),
    through: None,
  }
}

/// Create a `Many` relation through a junction table (many-to-many).
pub fn many_through(
  field_name: &str,
  target_table: &str,
  columns: &[(&str, &str)],
  junction_table: &str,
  junction_local: &str,
  junction_foreign: &str,
) -> RelationDef {
  RelationDef {
    field_name: field_name.to_owned(),
    kind: RelationKind::Many,
    target_table: target_table.to_owned(),
    join_columns: columns
      .iter()
      .map(|(local, foreign)| JoinColumn {
        local: (*local).to_owned(),
        foreign: (*foreign).to_owned(),
      })
      .collect(),
    through: Some(ThroughDef {
      junction_table: junction_table.to_owned(),
      local_column: junction_local.to_owned(),
      foreign_column: junction_foreign.to_owned(),
    }),
  }
}

/// Registry of relation definitions, keyed by source table name.
#[derive(Debug, Clone)]
pub struct RelationRegistry {
  entries: BTreeMap<String, Vec<RelationDef>>,
}

impl RelationRegistry {
  /// Create an empty registry.
  pub fn new() -> Self {
    Self {
      entries: BTreeMap::new(),
    }
  }

  /// Register relations for a source table (overwrites prior entry).
  pub fn register(&mut self, source_table: &str, relations: Vec<RelationDef>) {
    self.entries.insert(source_table.to_owned(), relations);
  }

  /// Relations for a source table, if any.
  pub fn get(&self, source_table: &str) -> Option<&[RelationDef]> {
    self.entries.get(source_table).map(Vec::as_slice)
  }
}

impl Default for RelationRegistry {
  fn default() -> Self {
    Self::new()
  }
}
