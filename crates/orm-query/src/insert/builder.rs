//! The [`InsertBuilder`] itself: columns, values, and dialect rendering.

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::Scalar;
use toolu_orm_core::query_column::Column;
use toolu_orm_core::value::Value;

use crate::where_clause::cfg_single_backend;

use super::conflict::{push_legacy_postgres_replace, ConflictMode};
use super::on_conflict::OnConflict;

cfg_single_backend! {
  use crate::exec_helpers::impl_execute;
}

pub struct InsertBuilder {
  /// Named by the `RETURNING` fetch methods for `QueryError::NotFound`.
  pub(super) table: String,
  columns: Vec<String>,
  /// One scalar per column, in the same order; a plain `set` stores a bind.
  values: Vec<Scalar>,
  conflict_mode: ConflictMode,
  conflict_cols: Vec<String>,
  /// Column names projected by `RETURNING`, in call order.
  returning: Vec<String>,
}

impl InsertBuilder {
  pub fn new(table: &str) -> Self {
    Self {
      table: table.to_owned(),
      columns: Vec::new(),
      values: Vec::new(),
      conflict_mode: ConflictMode::None,
      conflict_cols: Vec::new(),
      returning: Vec::new(),
    }
  }

  /// `"<column>"` bound to one value.
  pub fn set<T>(mut self, col: &Column<T>, val: impl Into<Value>) -> Self {
    self.columns.push(col.name.to_owned());
    self.values.push(Scalar::bind(val));
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
  /// [`fetch_one`](Self::fetch_one) / `fetch_optional` / `fetch_all` rather
  /// than `execute`: rusqlite refuses to `execute` a row-producing statement,
  /// and the other two drivers discard the rows. A conflict clause that took
  /// the `DO NOTHING` branch produces no row at all, so `fetch_optional`
  /// returns `None` there.
  pub fn returning<T>(mut self, col: &Column<T>) -> Self {
    self.returning.push(col.name.to_owned());
    self
  }

  /// Appends ` RETURNING "a", "b"` when any column was projected.
  fn push_returning(&self, sql: &mut String) {
    if self.returning.is_empty() {
      return;
    }
    let cols: Vec<String> = self.returning.iter().map(|c| format!(r#""{c}""#)).collect();
    sql.push_str(&format!(" RETURNING {}", cols.join(", ")));
  }

  pub fn to_sql_for(&self, dialect: Dialect) -> (String, Vec<Value>) {
    match dialect {
      Dialect::Sqlite => self.to_sql_sqlite(),
      Dialect::Postgres => self.to_sql_postgres(),
    }
  }

  pub fn to_sql(&self) -> (String, Vec<Value>) {
    self.to_sql_for(Dialect::CURRENT)
  }

  /// Appends `("a", "b") VALUES (<a>, <b>)` and returns the values bound, in
  /// the order their placeholders were written.
  fn push_columns_and_values(&self, sql: &mut String, dialect: Dialect) -> Vec<Value> {
    let col_list: Vec<String> = self.columns.iter().map(|c| format!(r#""{c}""#)).collect();
    sql.push_str(&format!(" ({}) VALUES (", col_list.join(", ")));

    let mut params: Vec<Value> = Vec::with_capacity(self.values.len());
    let mut rendered: Vec<String> = Vec::with_capacity(self.values.len());
    for value in &self.values {
      let start = params.len() + 1;
      let (fragment, value_params) = value.to_sql_fragment_for(start, dialect);
      params.extend(value_params);
      rendered.push(fragment);
    }

    sql.push_str(&rendered.join(", "));
    sql.push(')');
    params
  }

  fn to_sql_sqlite(&self) -> (String, Vec<Value>) {
    let mut sql = String::new();

    let keyword = match self.conflict_mode {
      ConflictMode::Replace => "INSERT OR REPLACE INTO",
      ConflictMode::Ignore => "INSERT OR IGNORE INTO",
      ConflictMode::None | ConflictMode::Clause(_) => "INSERT INTO",
    };

    sql.push_str(keyword);
    sql.push_str(&format!(r#" "{}""#, self.table));
    let mut params = self.push_columns_and_values(&mut sql, Dialect::Sqlite);

    if let ConflictMode::Clause(clause) = &self.conflict_mode {
      clause.push_sql(&mut sql, &mut params, Dialect::Sqlite);
    }
    self.push_returning(&mut sql);

    (sql, params)
  }

  fn to_sql_postgres(&self) -> (String, Vec<Value>) {
    let mut sql = String::new();

    sql.push_str(&format!(r#"INSERT INTO "{}""#, self.table));
    let mut params = self.push_columns_and_values(&mut sql, Dialect::Postgres);

    match &self.conflict_mode {
      ConflictMode::None => {},
      ConflictMode::Ignore => sql.push_str(" ON CONFLICT DO NOTHING"),
      ConflictMode::Replace => {
        push_legacy_postgres_replace(&mut sql, &self.columns, &self.conflict_cols);
      },
      ConflictMode::Clause(clause) => clause.push_sql(&mut sql, &mut params, Dialect::Postgres),
    }
    self.push_returning(&mut sql);

    (sql, params)
  }
}

cfg_single_backend! {
  impl_execute!(InsertBuilder, "INSERT");
}
