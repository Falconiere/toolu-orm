//! Set operations: two whole statements read as one result set.

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::value::Value;

use super::{Cte, SelectBuilder};

/// Which set operation attaches an arm to what came before it.
pub(super) enum SetOp {
  /// Duplicate rows collapse.
  Union,
  /// Every row of both arms.
  UnionAll,
}

impl SetOp {
  fn keyword(&self) -> &'static str {
    match self {
      Self::Union => "UNION",
      Self::UnionAll => "UNION ALL",
    }
  }
}

impl SelectBuilder {
  /// `<self> UNION <other>` — the merged rows, duplicates collapsed.
  ///
  /// This is what terminates a recursive walk over a cyclic graph: a node
  /// already in the working set is not re-expanded. Use
  /// [`Self::union_all`] when duplicates matter and the input cannot cycle.
  ///
  /// The arm contributes its select list, source, joins, `WHERE`, `GROUP BY`
  /// and `HAVING`. Its `ORDER BY`, `LIMIT` and `OFFSET` are **not** rendered —
  /// SQL has no place for them on an individual arm — so order and paginate on
  /// the builder `union` was called on, which bounds the whole compound. An
  /// arm's own CTEs are hoisted into the single `WITH` prefix.
  ///
  /// Order a compound by an **output** name — `OrderBy::alias_asc("id")` — not
  /// by a qualified column: the merged result has no table qualification, and
  /// Postgres rejects `ORDER BY "t"."id"` there with *missing FROM-clause
  /// entry* where SQLite tolerates it.
  pub fn union(mut self, other: SelectBuilder) -> Self {
    self.set_ops.push((SetOp::Union, other));
    self
  }

  /// `<self> UNION ALL <other>` — every row of both arms; see [`Self::union`].
  pub fn union_all(mut self, other: SelectBuilder) -> Self {
    self.set_ops.push((SetOp::UnionAll, other));
    self
  }

  /// Whether this builder renders more than one arm.
  pub(super) fn is_compound(&self) -> bool {
    !self.set_ops.is_empty()
  }

  /// `<core> [ UNION [ALL] <core> ]*`, every arm sharing one parameter vector.
  ///
  /// This builder's own core renders first; each *arm* then recurses through
  /// this same method rather than through [`SelectBuilder::push_core`], so a
  /// nested compound flattens — `a.union(b.union(c))` renders exactly what
  /// `a.union(b).union(c)` renders. Pushing only an arm's core would silently
  /// drop that arm's own arms.
  ///
  /// No clause ever leaves a trailing space, so the separator writes its own
  /// on both sides and the result cannot double up.
  pub(super) fn push_compound(&self, sql: &mut String, params: &mut Vec<Value>, dialect: Dialect) {
    self.push_core(sql, params, dialect);
    for (op, arm) in &self.set_ops {
      sql.push(' ');
      sql.push_str(op.keyword());
      sql.push(' ');
      arm.push_compound(sql, params, dialect);
    }
  }

  /// Every CTE this builder and its arms declare, in render order.
  pub(super) fn collect_ctes<'a>(&'a self, out: &mut Vec<&'a Cte>) {
    out.extend(self.ctes.iter());
    for (_, arm) in &self.set_ops {
      arm.collect_ctes(out);
    }
  }
}
