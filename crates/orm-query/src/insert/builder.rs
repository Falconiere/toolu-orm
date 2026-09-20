//! The [`InsertBuilder`] itself: the target, the column/value pairs, the
//! conflict policy and the `RETURNING` projection.
//!
//! The `SELECT` row source lives in `super::rows`, and rendering in
//! `super::statement`; both read these fields.

use toolu_orm_core::alias::TableRef;
use toolu_orm_core::expr::Scalar;
use toolu_orm_core::query_column::{tag_column_bind, Column};
use toolu_orm_core::value::Value;

use crate::where_clause::cfg_single_backend;

use super::conflict::ConflictMode;
use super::on_conflict::OnConflict;
use super::rows::SelectRows;

cfg_single_backend! {
  use crate::exec_helpers::impl_execute;
}

pub struct InsertBuilder {
  /// The `INSERT INTO` target, which may carry a database qualifier.
  pub(super) table: TableRef,
  pub(super) columns: Vec<String>,
  /// One scalar per column, in the same order; a plain `set` stores a bind.
  /// Unused, but kept, while a `SELECT` source is present.
  pub(super) values: Vec<Scalar>,
  /// When set, rows come from this statement instead of from `values`.
  pub(super) select: Option<SelectRows>,
  pub(super) conflict_mode: ConflictMode,
  pub(super) conflict_cols: Vec<String>,
  /// Column names projected by `RETURNING`, in call order.
  pub(super) returning: Vec<String>,
}

impl InsertBuilder {
  pub fn new(table: &str) -> Self {
    Self::into_table(table)
  }

  /// [`Self::new`] for a target that may carry a database qualifier or an
  /// alias.
  ///
  /// `TableRef::new("indexed_files").in_database("main")` renders
  /// `INSERT INTO "main"."indexed_files"` — two identifiers, where the dotted
  /// string `"main.indexed_files"` would be one. `&str`, `String`, `TableRef`
  /// and `&TableRef` all convert.
  pub fn into_table(table: impl Into<TableRef>) -> Self {
    Self {
      table: table.into(),
      columns: Vec::new(),
      values: Vec::new(),
      select: None,
      conflict_mode: ConflictMode::None,
      conflict_cols: Vec::new(),
      returning: Vec::new(),
    }
  }

  /// The target's base name, never its database qualifier or alias.
  ///
  /// This is what `QueryError::NotFound` carries, because it is what the
  /// caller named.
  pub fn table_name(&self) -> &str {
    self.table.table()
  }

  /// `"<column>"` bound to one value.
  pub fn set<T: 'static>(mut self, col: &Column<T>, val: impl Into<Value>) -> Self {
    self.columns.push(col.name.to_owned());
    self
      .values
      .push(Scalar::bind(tag_column_bind::<T>(val.into())));
    self
  }

  pub fn set_null<T>(mut self, col: &Column<T>) -> Self {
    self.columns.push(col.name.to_owned());
    self.values.push(Scalar::bind(Value::Null));
    self
  }

  /// `"<column>"` set to a computed value: `unixepoch()`, `coalesce(?, 'x')`,
  /// or anything else a [`Scalar`] spells. Its binds are numbered in column
  /// order alongside the plain ones.
  pub fn set_scalar<T>(mut self, col: &Column<T>, expr: Scalar) -> Self {
    self.columns.push(col.name.to_owned());
    self.values.push(expr);
    self
  }

  /// An explicit `ON CONFLICT (…) DO NOTHING | DO UPDATE SET …`, rendered
  /// identically on SQLite and Postgres.
  ///
  /// This is the form that preserves the conflicting row: unlike
  /// [`or_replace`](Self::or_replace) it updates in place, so columns it does
  /// not name keep their stored values and referencing rows are never
  /// cascaded away. It replaces any earlier `or_replace()` / `or_ignore()`
  /// and is replaced by a later one — a statement carries exactly one
  /// conflict policy — and it ignores
  /// [`conflict_columns`](Self::conflict_columns), whose target belongs to
  /// the legacy shorthand.
  pub fn on_conflict(mut self, clause: OnConflict) -> Self {
    self.conflict_mode = ConflictMode::Clause(clause);
    self
  }

  /// SQLite `INSERT OR REPLACE`; Postgres `ON CONFLICT (…) DO UPDATE SET …`
  /// over every non-target column.
  ///
  /// SQLite's form **deletes** the conflicting row and inserts a new one, so
  /// every column this builder does not set falls back to its default and
  /// every `ON DELETE CASCADE` child row is deleted with it. Reach for
  /// [`on_conflict`](Self::on_conflict) when the stored row must survive.
  pub fn or_replace(mut self) -> Self {
    self.conflict_mode = ConflictMode::Replace;
    self
  }

  pub fn or_ignore(mut self) -> Self {
    self.conflict_mode = ConflictMode::Ignore;
    self
  }

  /// Columns that form the `ON CONFLICT (...)` target of the Postgres
  /// rendering of [`or_replace`](Self::or_replace).
  ///
  /// It feeds only that shorthand: SQLite's `INSERT OR REPLACE` /
  /// `INSERT OR IGNORE` take no target, and
  /// [`on_conflict`](Self::on_conflict) carries its own. If unset, the first
  /// inserted column is used.
  pub fn conflict_columns(mut self, cols: &[&str]) -> Self {
    self.conflict_cols = cols.iter().map(|c| (*c).to_owned()).collect();
    self
  }

  /// Appends one column to `RETURNING "a", "b"`, in call order.
  ///
  /// The list is rendered last and unqualified, which both engines accept,
  /// and it binds nothing. Read the projected rows with
  /// `fetch_one` / `fetch_optional` / `fetch_all` rather
  /// than `execute`: rusqlite refuses to `execute` a row-producing statement,
  /// and the other two drivers discard the rows. A conflict clause that took
  /// the `DO NOTHING` branch produces no row at all, so `fetch_optional`
  /// returns `None` there.
  pub fn returning<T>(mut self, col: &Column<T>) -> Self {
    self.returning.push(col.name.to_owned());
    self
  }
}

cfg_single_backend! {
  impl_execute!(InsertBuilder, "INSERT");
}
