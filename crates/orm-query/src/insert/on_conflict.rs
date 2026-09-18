//! [`OnConflict`] — an explicit `ON CONFLICT (…) DO NOTHING | DO UPDATE SET …`.

use toolu_orm_core::alias::quote_ident;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::Scalar;
use toolu_orm_core::query_column::Column;
use toolu_orm_core::value::Value;

/// What the clause does once its target matches.
///
/// Private, so a later feature (a `DO UPDATE … WHERE` guard, a constraint
/// target) can add a variant without breaking the published surface.
enum ConflictAction {
  DoNothing,
  /// Assigned column name and the scalar it is set to, in call order.
  DoUpdate(Vec<(String, Scalar)>),
}

/// A conflict target and the action to take on it, spelled the same way on
/// SQLite and Postgres: `ON CONFLICT ("a", "b") DO UPDATE SET "c" = …`.
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
}

impl OnConflict {
  /// Starts the target with one column; the action is `DO NOTHING` until an
  /// assignment is added.
  pub fn column<T>(col: &Column<T>) -> Self {
    Self {
      target: vec![col.name.to_owned()],
      action: ConflictAction::DoNothing,
    }
  }

  /// Adds another column to a composite target, in call order — the unique
  /// index `(repo, path)` is `column(&REPO).and_column(&PATH)`.
  pub fn and_column<T>(mut self, col: &Column<T>) -> Self {
    self.target.push(col.name.to_owned());
    self
  }

  /// `DO NOTHING`, discarding every assignment recorded so far.
  pub fn do_nothing(mut self) -> Self {
    self.action = ConflictAction::DoNothing;
    self
  }

  /// `DO UPDATE SET "<col>" = ?N` — one bound value.
  pub fn set<T>(self, col: &Column<T>, val: impl Into<Value>) -> Self {
    self.assign(col.name, Scalar::bind(val))
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

  /// Appends ` ON CONFLICT (…) DO …` to `sql`, pushing the assignments' bound
  /// values onto the statement's `params` in emission order.
  ///
  /// `params` already holds the `VALUES` binds, so each assignment numbers
  /// from `params.len() + 1` — the statement-absolute index of its first
  /// placeholder, which is what [`Scalar::to_sql_fragment_for`] expects.
  pub(super) fn push_sql(&self, sql: &mut String, params: &mut Vec<Value>, dialect: Dialect) {
    let target: Vec<String> = self.target.iter().map(|c| quote_ident(c)).collect();
    sql.push_str(&format!(" ON CONFLICT ({}) DO ", target.join(", ")));

    match &self.action {
      ConflictAction::DoNothing => sql.push_str("NOTHING"),
      ConflictAction::DoUpdate(sets) => {
        let mut parts: Vec<String> = Vec::with_capacity(sets.len());
        for (column, value) in sets {
          let start = params.len() + 1;
          let (fragment, value_params) = value.to_sql_fragment_for(start, dialect);
          params.extend(value_params);
          parts.push(format!("{} = {fragment}", quote_ident(column)));
        }
        sql.push_str(&format!("UPDATE SET {}", parts.join(", ")));
      },
    }
  }
}
