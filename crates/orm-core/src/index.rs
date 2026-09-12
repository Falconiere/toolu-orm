use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexDef {
  pub name: String,
  pub columns: Vec<String>,
  pub unique: bool,
  /// Partial-index predicate, rendered as `WHERE <predicate>` when set.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub where_clause: Option<String>,
}
