use serde::{Deserialize, Serialize};

use crate::column::ColumnDef;
use crate::index::IndexDef;

/// How the database creates the table.
///
/// `Virtual` covers every `CREATE VIRTUAL TABLE … USING <module>(<args>)`
/// form — `fts5`, `vec0`, `rtree`, or a module the application registered
/// itself. `args` are already-rendered SQL arguments, joined with `", "`
/// inside the module parentheses, so the schema layer stays module-agnostic
/// and each module gets its own builder (see [`crate::fts5`]).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableDef {
  pub name: String,
  pub columns: Vec<ColumnDef>,
  #[serde(default)]
  pub indexes: Vec<IndexDef>,
  #[serde(default)]
  pub strict: bool,
  /// Defaults to [`TableKind::Ordinary`], so snapshots written before virtual
  /// tables existed still deserialize.
  #[serde(default, skip_serializing_if = "TableKind::is_ordinary")]
  pub kind: TableKind,
}

impl TableDef {
  pub fn find_column(&self, name: &str) -> Option<&ColumnDef> {
    self.columns.iter().find(|c| c.name == name)
  }

  #[must_use]
  pub fn is_virtual(&self) -> bool {
    !self.kind.is_ordinary()
  }
}

pub trait TableSchema {
  fn table_def() -> TableDef;
}
