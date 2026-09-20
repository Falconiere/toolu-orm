//! [`OnConflict`] — `ON CONFLICT (…) [WHERE …] DO NOTHING | DO UPDATE SET … [WHERE …]`.

use toolu_orm_core::alias::quote_ident;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::{BoundParams, Expr, Scalar};
use toolu_orm_core::query_column::{tag_column_bind, Column};
use toolu_orm_core::value::Value;

use crate::where_clause::append_conjuncts_for;

/// What the clause does once its target matches.
enum ConflictAction {
  DoNothing,
  /// Assigned column name and the scalar it is set to, in call order.
  DoUpdate(Vec<(String, Scalar)>),
}

/// A conflict target and the action to take on it, spelled the same way on
/// SQLite and Postgres: `ON CONFLICT ("a", "b") WHERE … DO UPDATE SET "c" = … WHERE …`.
///
/// This is the form to reach for instead of
/// [`or_replace`](super::InsertBuilder::or_replace): `INSERT OR REPLACE`
/// deletes the conflicting row and inserts a new one, which resets every
/// column the caller did not name and fires `ON DELETE CASCADE` on every
/// referencing row. `DO UPDATE` writes only the columns it lists.
///
/// The action is `DO NOTHING` until the first assignment is recorded.
///
/// ```
/// use toolu_orm_core::column::{Integer, Text};
/// use toolu_orm_core::dialect::Dialect;
/// use toolu_orm_core::expr::Scalar;
/// use toolu_orm_core::query_column::Column;
/// use toolu_orm_query::insert::{InsertBuilder, OnConflict};
///
/// const ID: Column<Text> = Column::new("feedback", "memory_id");
/// const USED: Column<Integer> = Column::new("feedback", "used_count");
///
/// let (sql, params) = InsertBuilder::new("feedback")
///   .set(&ID, "m1")
///   .set(&USED, 1_i64)
///   .on_conflict(
///     OnConflict::column(&ID).set_scalar(&USED, Scalar::col(&USED) + Scalar::bind(1_i64)),
///   )
///   .to_sql_for(Dialect::Sqlite);
///
/// assert_eq!(
///   sql,
///   r#"INSERT INTO "feedback" ("memory_id", "used_count") VALUES (?1, ?2) "#.to_owned()
///     + r#"ON CONFLICT ("memory_id") DO UPDATE SET "used_count" = ("feedback"."used_count" + ?3)"#
/// );
/// assert_eq!(params.len(), 3);
/// ```
pub struct OnConflict {
  target: Vec<String>,
  action: ConflictAction,
  /// Partial-index predicate, rendered between the target and `DO`.
  target_where: Vec<Expr>,
  /// `DO UPDATE` guard. Rendered only for that action; [`Self::do_nothing`]
  /// clears it.
  update_where: Vec<Expr>,
}

impl OnConflict {
  /// Starts the target with one column; the action is `DO NOTHING` until an
  /// assignment is added.
  pub fn column<T>(col: &Column<T>) -> Self {
    Self {
      target: vec![col.name.to_owned()],
      action: ConflictAction::DoNothing,
      target_where: Vec::new(),
      update_where: Vec::new(),
    }
  }

  /// Adds another column to a composite target, in call order — the unique
  /// index `(repo, path)` is `column(&REPO).and_column(&PATH)`.
  pub fn and_column<T>(mut self, col: &Column<T>) -> Self {
    self.target.push(col.name.to_owned());
    self
  }

  /// `ON CONFLICT (…) WHERE <expr>` — another conjunct of the partial-index
  /// predicate, `AND`-joined in call order.
  ///
  /// The predicate is what names a partial unique index: it has to be that
  /// index's own expression (`deleted = 0`, not a bound parameter standing in
  /// for `0`), because both engines infer the index from it. It is rendered
  /// before `DO`, so any bind it does carry is numbered after the `VALUES`
  /// binds and before the `DO UPDATE` assignments.
  pub fn where_target(mut self, expr: Expr) -> Self {
    self.target_where.push(expr);
    self
  }

  /// `DO UPDATE SET … WHERE <expr>` — another conjunct of the update guard,
  /// `AND`-joined in call order.
  ///
  /// A false guard skips the write the way `DO NOTHING` does: no row changes
  /// and `RETURNING` projects nothing. The guard binds after the assignments,
  /// because it is rendered after them. It is not part of `DO NOTHING`, so
  /// [`do_nothing`](Self::do_nothing) discards it and a clause that never
  /// records an assignment does not render it or bind it.
  pub fn where_update(mut self, expr: Expr) -> Self {
    self.update_where.push(expr);
    self
  }

  /// `DO NOTHING`, discarding every assignment and every update guard
  /// recorded so far. The index predicate stays: it belongs to the target.
  pub fn do_nothing(mut self) -> Self {
    self.action = ConflictAction::DoNothing;
    self.update_where.clear();
    self
  }

  /// `DO UPDATE SET "<col>" = ?N` — one bound value.
  pub fn set<T: 'static>(self, col: &Column<T>, val: impl Into<Value>) -> Self {
    self.assign(col.name, Scalar::bind(tag_column_bind::<T>(val.into())))
  }

  /// `DO UPDATE SET "<col>" = <scalar>` — a computed assignment.
  ///
  /// Its placeholders are numbered after every `VALUES` placeholder, because
  /// the conflict clause is rendered last: `"used_count" =
  /// ("feedback"."used_count" + ?3)` on a statement whose `VALUES` bound two.
  /// Inside this clause [`Scalar::col`] names the **existing** row and
  /// [`Scalar::excluded`] the one the `INSERT` proposed.
  pub fn set_scalar<T>(self, col: &Column<T>, expr: Scalar) -> Self {
    self.assign(col.name, expr)
  }

  /// `DO UPDATE SET "<col>" = "excluded"."<col>"` — take the proposed value.
  pub fn set_excluded<T>(self, col: &Column<T>) -> Self {
    self.assign(col.name, Scalar::excluded(col))
  }

  fn assign(mut self, column: &str, value: Scalar) -> Self {
    let entry = (column.to_owned(), value);
    match &mut self.action {
      ConflictAction::DoUpdate(sets) => sets.push(entry),
      ConflictAction::DoNothing => self.action = ConflictAction::DoUpdate(vec![entry]),
    }
    self
  }

  /// Appends ` ON CONFLICT (…) [WHERE …] DO …` to `sql`.
  ///
  /// `params` already holds the `VALUES` binds. The index predicate is
  /// rendered next, then each assignment, then the update guard, and each of
  /// those numbers from [`BoundParams::next_index`]. An assignment built from
  /// a [`SharedBind`](toolu_orm_core::expr::SharedBind) the `VALUES` already
  /// bound reuses that placeholder instead of adding one. The update guard is
  /// omitted entirely when the action is `DO NOTHING`.
  pub(super) fn push_sql(&self, sql: &mut String, params: &mut BoundParams, dialect: Dialect) {
    let target: Vec<String> = self.target.iter().map(|c| quote_ident(c)).collect();
    sql.push_str(&format!(" ON CONFLICT ({})", target.join(", ")));
    append_conjuncts_for(&self.target_where, " WHERE ", sql, params, dialect);
    sql.push_str(" DO ");

    match &self.action {
      ConflictAction::DoNothing => sql.push_str("NOTHING"),
      ConflictAction::DoUpdate(sets) => {
        let mut parts: Vec<String> = Vec::with_capacity(sets.len());
        for (column, value) in sets {
          let fragment = value.render_into(params, dialect);
          parts.push(format!("{} = {fragment}", quote_ident(column)));
        }
        sql.push_str(&format!("UPDATE SET {}", parts.join(", ")));
        append_conjuncts_for(&self.update_where, " WHERE ", sql, params, dialect);
      },
    }
  }
}
