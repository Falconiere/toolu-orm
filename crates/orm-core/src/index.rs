use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexDef {
  pub name: String,
  pub columns: Vec<String>,
  pub unique: bool,
}
