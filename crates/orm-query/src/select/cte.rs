//! Common table expressions: the `WITH` prefix a statement carries.

use toolu_orm_core::alias::{quote_ident, TableRef};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::BoundParams;

use super::SelectBuilder;

/// One named query in a `WITH` prefix.
///
/// The body is an ordinary [`SelectBuilder`], so a recursive walk is just a
/// body whose arms are `anchor.union(step)`. `UNION` deduplicates whole rows;
/// when depth is projected, the step still needs a depth bound or cycle guard.
///
/// ```ignore
/// let walk = Cte::new("walk", anchor.union(step))
///   .columns(&["kind", "id", "depth"])
///   .recursive();
/// let rows = SelectBuilder::from_table(walk.table_ref()).with(walk);
/// ```
pub struct Cte {
  name: String,
  columns: Vec<String>,
  query: SelectBuilder,
  recursive: bool,
}

impl Cte {
  /// A CTE named `name` whose body is `query`.
  pub fn new(name: impl Into<String>, query: SelectBuilder) -> Self {
    Self {
      name: name.into(),
      columns: Vec::new(),
      query,
      recursive: false,
    }
  }

  /// An explicit output column list: `"walk"("kind", "id", "depth")`.
  ///
  /// Needed whenever the body's arms do not already agree on output names —
  /// which is the usual case for a recursive walk, where the anchor projects
  /// literals and the step projects columns.
  pub fn columns(mut self, columns: &[&str]) -> Self {
    self.columns = columns.iter().map(|c| (*c).to_owned()).collect();
    self
  }

  /// Marks this CTE self-referential.
  ///
  /// One such CTE makes the whole prefix `WITH RECURSIVE`, which both engines
  /// accept, including for the non-recursive members beside it. Whether
  /// the body actually refers to itself is not policed here; the engine
  /// reports a recursive term that is not in a compound.
  pub fn recursive(mut self) -> Self {
    self.recursive = true;
    self
  }

  /// The name this CTE is declared under.
  pub fn name(&self) -> &str {
    &self.name
  }

  /// This CTE as a `FROM` / `JOIN` source: a plain quoted identifier.
  pub fn table_ref(&self) -> TableRef {
    TableRef::new(self.name.clone())
  }

  /// `"name"[("col", …)] AS (<body>)`.
  fn push(&self, sql: &mut String, params: &mut BoundParams, dialect: Dialect) {
    sql.push_str(&quote_ident(&self.name));
    if !self.columns.is_empty() {
      let columns: Vec<String> = self.columns.iter().map(|c| quote_ident(c)).collect();
      sql.push_str(&format!("({})", columns.join(", ")));
    }
    sql.push_str(" AS (");
    self
      .query
      .push_statement(sql, params, dialect, self.query.limit_val);
    sql.push(')');
  }
}

impl SelectBuilder {
  /// Prepends one common table expression; call order is render order.
  ///
  /// The CTE is visible to this statement, to its set-operation arms, and to
  /// the CTEs declared after it. Reference it with
  /// [`Cte::table_ref`](Cte::table_ref) — or by name, since a CTE is an
  /// ordinary quoted identifier in a `FROM` slot.
  pub fn with(mut self, cte: Cte) -> Self {
    self.ctes.push(cte);
    self
  }

  /// `WITH [RECURSIVE] <cte>, … `, or nothing when there are none.
  ///
  /// Rendered before `SELECT`, so CTE bodies take the statement's low bind
  /// indices. A compound has one prefix, so the arms' CTEs are hoisted into it
  /// — dropping them would leave an arm naming a relation nothing declared.
  pub(super) fn push_with_prefix(
    &self,
    sql: &mut String,
    params: &mut BoundParams,
    dialect: Dialect,
  ) {
    let mut ctes: Vec<&Cte> = Vec::new();
    self.collect_ctes(&mut ctes);
    if ctes.is_empty() {
      return;
    }

    let recursive = ctes.iter().any(|cte| cte.recursive);
    sql.push_str(if recursive {
      "WITH RECURSIVE "
    } else {
      "WITH "
    });
    for (index, cte) in ctes.iter().enumerate() {
      if index > 0 {
        sql.push_str(", ");
      }
      cte.push(sql, params, dialect);
    }
    sql.push(' ');
  }
}
