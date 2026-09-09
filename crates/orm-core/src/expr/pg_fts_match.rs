//! Postgres `document @@ query_fn(config, $N)` as an [`Expr`].

use crate::value::Value;

use super::types::{Expr, ExprKind};

impl Expr {
  /// Build a [`ExprKind::TsMatch`] node. Callers must already have refused
  /// non-Postgres dialects and validated `config`.
  pub(crate) fn ts_match_target(
    document: String,
    query_fn: &'static str,
    config: String,
    pattern: Value,
  ) -> Self {
    Self {
      kind: ExprKind::TsMatch {
        document,
        query_fn,
        config,
        pattern,
      },
    }
  }
}
