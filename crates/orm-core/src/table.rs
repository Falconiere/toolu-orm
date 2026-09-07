use crate::column::ColumnDef;
use crate::index::IndexDef;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableDef {
  pub name: String,
  pub columns: Vec<ColumnDef>,
  #[serde(default)]
  pub indexes: Vec<IndexDef>,
  #[serde(default)]
  pub strict: bool,
}

impl TableDef {
  pub fn find_column(&self, name: &str) -> Option<&ColumnDef> {
    self.columns.iter().find(|c| c.name == name)
  }
}

pub trait TableSchema {
  fn table_def() -> TableDef;
}
