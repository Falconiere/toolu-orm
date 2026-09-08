//! The closed type sets `vec0` accepts in each column position.
//!
//! One enum per position rather than one shared list: `vec0` takes only `text`
//! and `integer` for a key, has no `blob` metadata column and no `boolean`
//! auxiliary column, so a shared type would need a rendering arm for a
//! combination the module rejects.

use crate::column::ColumnType;

/// How `vec0` measures the distance between two vectors. `l2` is its default,
/// so it is still rendered explicitly when asked for: the argument list is
/// what the diff compares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DistanceMetric {
  L2,
  Cosine,
  L1,
}

impl DistanceMetric {
  #[must_use]
  pub fn as_vec0_sql(self) -> &'static str {
    match self {
      Self::L2 => "l2",
      Self::Cosine => "cosine",
      Self::L1 => "l1",
    }
  }
}

/// The type of a `vec0` primary key or partition key column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vec0KeyType {
  Text,
  Integer,
}

impl Vec0KeyType {
  #[must_use]
  pub fn as_vec0_sql(self) -> &'static str {
    match self {
      Self::Text => "text",
      Self::Integer => "integer",
    }
  }

  #[must_use]
  pub fn column_type(self) -> ColumnType {
    match self {
      Self::Text => ColumnType::Text,
      Self::Integer => ColumnType::Integer,
    }
  }
}

/// The type of a metadata column — one that can appear in the `WHERE` clause
/// of a KNN query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vec0MetadataType {
  Text,
  Integer,
  Float,
  Boolean,
}

impl Vec0MetadataType {
  #[must_use]
  pub fn as_vec0_sql(self) -> &'static str {
    match self {
      Self::Text => "text",
      Self::Integer => "integer",
      Self::Float => "float",
      Self::Boolean => "boolean",
    }
  }

  #[must_use]
  pub fn column_type(self) -> ColumnType {
    match self {
      Self::Text => ColumnType::Text,
      Self::Integer => ColumnType::Integer,
      Self::Float => ColumnType::Real,
      Self::Boolean => ColumnType::Boolean,
    }
  }
}

/// The type of an auxiliary column — stored beside the index and returned by a
/// KNN query, but never filtered on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vec0AuxiliaryType {
  Text,
  Integer,
  Float,
  Blob,
}

impl Vec0AuxiliaryType {
  #[must_use]
  pub fn as_vec0_sql(self) -> &'static str {
    match self {
      Self::Text => "text",
      Self::Integer => "integer",
      Self::Float => "float",
      Self::Blob => "blob",
    }
  }

  #[must_use]
  pub fn column_type(self) -> ColumnType {
    match self {
      Self::Text => ColumnType::Text,
      Self::Integer => ColumnType::Integer,
      Self::Float => ColumnType::Real,
      Self::Blob => ColumnType::Blob,
    }
  }
}
