//! Builder that turns declared columns into a `vec0` [`TableDef`].

use crate::column::VectorElement;
use crate::error::DbCoreError;
use crate::table::{TableDef, TableKind};

use super::ident::is_vec0_ident;
use super::role::{Vec0Column, Vec0Role};
use super::types::{DistanceMetric, Vec0AuxiliaryType, Vec0KeyType, Vec0MetadataType};

/// The `sqlite-vec` module name for vector tables.
pub const VEC0_MODULE: &str = "vec0";

/// Assembles `CREATE VIRTUAL TABLE … USING vec0(…)` without hand-writing the
/// module arguments.
///
/// Columns render in declaration order, which is the order `vec0` assigns
/// them, so the same schema always produces the same arguments and an
/// unchanged schema never looks changed to the diff.
///
/// ```
/// use toolu_orm_core::column::VectorElement;
/// use toolu_orm_core::vec0::{DistanceMetric, Vec0KeyType, Vec0Table};
///
/// let table = Vec0Table::new("memory_vec")
///   .primary_key("memory_id", Vec0KeyType::Text)
///   .vector_metric("embedding", VectorElement::Float, 1024, DistanceMetric::Cosine)
///   .build()?;
///
/// assert_eq!(table.kind.module(), Some("vec0"));
/// assert_eq!(table.kind.args()[1], "embedding float[1024] distance_metric=cosine");
/// # Ok::<(), toolu_orm_core::error::DbCoreError>(())
/// ```
#[derive(Debug, Default)]
pub struct Vec0Table {
  name: String,
  columns: Vec<Vec0Column>,
}

impl Vec0Table {
  #[must_use]
  pub fn new(name: impl Into<String>) -> Self {
    Self {
      name: name.into(),
      columns: Vec::new(),
    }
  }

  /// The single primary key column; `vec0` allows at most one.
  #[must_use]
  pub fn primary_key(self, name: impl Into<String>, key_type: Vec0KeyType) -> Self {
    self.push(name, Vec0Role::PrimaryKey(key_type))
  }

  /// A vector column of `dim` elements, left at `vec0`'s default `l2` metric.
  #[must_use]
  pub fn vector(self, name: impl Into<String>, element: VectorElement, dim: u32) -> Self {
    self.push(
      name,
      Vec0Role::Vector {
        element,
        dim,
        metric: None,
      },
    )
  }

  /// A vector column with an explicit `distance_metric`, which `vec0` rejects
  /// on a `bit` vector — [`Self::build`] refuses that pair.
  #[must_use]
  pub fn vector_metric(
    self,
    name: impl Into<String>,
    element: VectorElement,
    dim: u32,
    metric: DistanceMetric,
  ) -> Self {
    self.push(
      name,
      Vec0Role::Vector {
        element,
        dim,
        metric: Some(metric),
      },
    )
  }

  /// A partition key: `vec0` shards the index on it and pre-filters an `=`
  /// constraint against it during a KNN search.
  #[must_use]
  pub fn partition_key(self, name: impl Into<String>, key_type: Vec0KeyType) -> Self {
    self.push(name, Vec0Role::PartitionKey(key_type))
  }

  /// A metadata column, usable in the `WHERE` clause of a KNN query.
  #[must_use]
  pub fn metadata(self, name: impl Into<String>, metadata_type: Vec0MetadataType) -> Self {
    self.push(name, Vec0Role::Metadata(metadata_type))
  }

  /// An auxiliary (`+`) column: returned by a KNN query, never filtered on.
  #[must_use]
  pub fn auxiliary(self, name: impl Into<String>, aux_type: Vec0AuxiliaryType) -> Self {
    self.push(name, Vec0Role::Auxiliary(aux_type))
  }

  /// The finished definition.
  ///
  /// # Errors
  ///
  /// [`DbCoreError::InvalidVec0Identifier`] when the table name or a column
  /// name is one `vec0`'s scanner cannot read, and
  /// [`DbCoreError::Vec0BitDistanceMetric`] for a `bit` vector given a metric.
  /// Both are rendering failures rather than database failures: `vec0` cannot
  /// quote, so there is no escaped form to fall back on.
  pub fn build(self) -> Result<TableDef, DbCoreError> {
    if !is_vec0_ident(&self.name) {
      return Err(DbCoreError::InvalidVec0Identifier {
        context: "table name",
        ident: self.name,
      });
    }
    for column in &self.columns {
      if !is_vec0_ident(&column.name) {
        return Err(DbCoreError::InvalidVec0Identifier {
          context: "column name",
          ident: column.name.clone(),
        });
      }
      if column.bit_vector_with_metric() {
        return Err(DbCoreError::Vec0BitDistanceMetric {
          column: column.name.clone(),
        });
      }
    }
    Ok(self.render())
  }

  /// The finished definition without the checks in [`Self::build`].
  ///
  /// `#[vec0_table]` calls this: it runs the same checks at expansion time,
  /// where a bad name is a compile error pointing at the offending field
  /// rather than a `Result` that `TableSchema::table_def` cannot return.
  #[must_use]
  pub fn build_prevalidated(self) -> TableDef {
    self.render()
  }

  fn push(mut self, name: impl Into<String>, role: Vec0Role) -> Self {
    self.columns.push(Vec0Column {
      name: name.into(),
      role,
    });
    self
  }

  fn render(self) -> TableDef {
    let args: Vec<String> = self.columns.iter().map(Vec0Column::arg).collect();
    let columns = self.columns.iter().map(Vec0Column::column_def).collect();
    TableDef {
      name: self.name,
      columns,
      indexes: Vec::new(),
      strict: false,
      kind: TableKind::virtual_table(VEC0_MODULE, args),
    }
  }
}
