//! What one declared column is to `vec0`, and how it renders.
//!
//! The five roles are the five shapes the module's constructor recognises, in
//! the order it tries them: a vector, a partition key, a primary key, an
//! auxiliary column, and — anything left — a metadata column.

use crate::column::{ColumnDef, ColumnType, VectorElement};

use super::types::{DistanceMetric, Vec0AuxiliaryType, Vec0KeyType, Vec0MetadataType};

#[derive(Debug)]
pub(super) enum Vec0Role {
  PrimaryKey(Vec0KeyType),
  Vector {
    element: VectorElement,
    dim: u32,
    metric: Option<DistanceMetric>,
  },
  PartitionKey(Vec0KeyType),
  Metadata(Vec0MetadataType),
  Auxiliary(Vec0AuxiliaryType),
}

#[derive(Debug)]
pub(super) struct Vec0Column {
  pub(super) name: String,
  pub(super) role: Vec0Role,
}

impl Vec0Column {
  /// One `vec0` constructor argument. Identifiers are bare: the module's
  /// scanner rejects a quote, so [`super::ident::is_vec0_ident`] guards the
  /// name instead.
  pub(super) fn arg(&self) -> String {
    let name = &self.name;
    match &self.role {
      Vec0Role::PrimaryKey(key_type) => format!("{name} {} primary key", key_type.as_vec0_sql()),
      Vec0Role::Vector {
        element,
        dim,
        metric,
      } => {
        let head = format!("{name} {}[{dim}]", element.as_vec0_sql());
        match metric {
          Some(metric) => format!("{head} distance_metric={}", metric.as_vec0_sql()),
          None => head,
        }
      },
      Vec0Role::PartitionKey(key_type) => {
        format!("{name} {} partition key", key_type.as_vec0_sql())
      },
      Vec0Role::Metadata(metadata_type) => format!("{name} {}", metadata_type.as_vec0_sql()),
      Vec0Role::Auxiliary(aux_type) => format!("+{name} {}", aux_type.as_vec0_sql()),
    }
  }

  /// The schema-layer column. The dimension and element type live here as
  /// well as in the rendered argument, so the diff sees a changed dimension as
  /// a changed column rather than only as changed module arguments.
  pub(super) fn column_def(&self) -> ColumnDef {
    let (column_type, primary_key) = match &self.role {
      Vec0Role::PrimaryKey(key_type) => (key_type.column_type(), true),
      Vec0Role::Vector { element, dim, .. } => (
        ColumnType::Vector {
          element: *element,
          dim: *dim,
        },
        false,
      ),
      Vec0Role::PartitionKey(key_type) => (key_type.column_type(), false),
      Vec0Role::Metadata(metadata_type) => (metadata_type.column_type(), false),
      Vec0Role::Auxiliary(aux_type) => (aux_type.column_type(), false),
    };
    ColumnDef {
      name: self.name.clone(),
      column_type,
      primary_key,
      not_null: false,
      default: None,
      unique: false,
      references: None,
      on_delete: None,
      on_update: None,
      check: None,
      unindexed: false,
      autoincrement: false,
    }
  }

  /// `vec0` rejects `distance_metric` on a bit vector: Hamming distance is the
  /// only thing it can mean.
  pub(super) fn bit_vector_with_metric(&self) -> bool {
    matches!(
      self.role,
      Vec0Role::Vector {
        element: VectorElement::Bit,
        metric: Some(_),
        ..
      }
    )
  }
}
