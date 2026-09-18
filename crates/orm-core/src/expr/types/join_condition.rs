//! `left = right` pair for `JOIN ... ON`.

/// `left = right` pair for `JOIN ... ON`.
pub struct JoinCondition {
  pub(crate) left: String,
  pub(crate) right: String,
}

impl JoinCondition {
  pub fn to_sql(&self) -> String {
    format!("{} = {}", self.left, self.right)
  }
}
