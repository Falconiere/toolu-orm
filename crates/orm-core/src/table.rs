use serde::{Deserialize, Serialize};

use crate::column::ColumnDef;
use crate::fts5::Fts5Sync;
use crate::index::IndexDef;
use crate::policy::RowSecurity;

/// How the database creates the table.
///
/// `Virtual` covers every `CREATE VIRTUAL TABLE … USING <module>(<args>)`
/// form — `fts5`, `vec0`, `rtree`, or a module the application registered
/// itself. `args` are already-rendered SQL arguments, joined with `", "`
/// inside the module parentheses, so the schema layer stays module-agnostic
/// and each module gets its own builder (see [`crate::fts5`]).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub enum TableKind {
  #[default]
  Ordinary,
  Virtual {
    module: String,
    args: Vec<String>,
  },
}

impl TableKind {
  /// Builds a virtual-table kind from a module name and rendered arguments.
  pub fn virtual_table(module: impl Into<String>, args: Vec<String>) -> Self {
    Self::Virtual {
      module: module.into(),
      args,
    }
  }

  /// True for a plain `CREATE TABLE`.
  #[must_use]
  pub fn is_ordinary(&self) -> bool {
    matches!(self, Self::Ordinary)
  }

  /// The module name for a virtual table, `None` for an ordinary one.
  #[must_use]
  pub fn module(&self) -> Option<&str> {
    match self {
      Self::Ordinary => None,
      Self::Virtual { module, .. } => Some(module),
    }
  }

  /// The rendered module arguments; empty for an ordinary table.
  #[must_use]
  pub fn args(&self) -> &[String] {
    match self {
      Self::Ordinary => &[],
      Self::Virtual { args, .. } => args,
    }
  }
}

/// One table as the schema declares it: the unit the registry, the snapshot
/// and the diff all work in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableDef {
  pub name: String,
  pub columns: Vec<ColumnDef>,
  #[serde(default)]
  pub indexes: Vec<IndexDef>,
  /// Table-level composite primary key column names. Empty means the key is
  /// declared per-column via [`ColumnDef::primary_key`]. Defaulted so older
  /// snapshots still deserialize.
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub primary_key: Vec<String>,
  #[serde(default)]
  pub strict: bool,
  /// Defaults to [`TableKind::Ordinary`], so snapshots written before virtual
  /// tables existed still deserialize.
  #[serde(default, skip_serializing_if = "TableKind::is_ordinary")]
  pub kind: TableKind,
  /// Set only by an external-content FTS5 table that opted into generated
  /// synchronization triggers; `None` for every other table, so a snapshot
  /// written before this existed is unchanged.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub fts5_sync: Option<Fts5Sync>,
  /// Postgres row-level security, when the table opts in; `None` for every
  /// other table, so a snapshot written before this existed is unchanged.
  /// SQLite has no equivalent and renders it as a comment.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub row_security: Option<RowSecurity>,
}

impl TableDef {
  /// The column declared under `name`, if the table has one.
  pub fn find_column(&self, name: &str) -> Option<&ColumnDef> {
    self.columns.iter().find(|c| c.name == name)
  }

  /// True for any `CREATE VIRTUAL TABLE` form.
  #[must_use]
  pub fn is_virtual(&self) -> bool {
    !self.kind.is_ordinary()
  }
}

/// Implemented by every declared table, by hand or through `#[table]`.
pub trait TableSchema {
  /// The table's declaration.
  fn table_def() -> TableDef;
}
