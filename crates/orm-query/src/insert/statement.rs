//! The full `INSERT` statement: how [`InsertBuilder`] renders its clauses and
//! numbers their parameters.
//!
//! One rule governs the numbering, the same one every other builder in this
//! workspace follows: position lives in the statement-wide
//! [`BoundParams`](toolu_orm_core::expr::BoundParams), and each clause takes
//! `next_index()` at the moment it writes. A caller never adds an offset on
//! top of it, which would double-count, so clauses can be added, removed or
//! reordered without renumbering anything after them.
//!
//! A [`SharedBind`](toolu_orm_core::expr::SharedBind) is the one thing that
//! may not advance the length: a repeat occurrence renders the index it
//! already took.

use toolu_orm_core::alias::quote_ident;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::BoundParams;
use toolu_orm_core::value::Value;

use super::conflict::{push_legacy_postgres_replace, ConflictMode};
use super::InsertBuilder;
use crate::where_clause::append_returning;

impl InsertBuilder {
  /// The whole statement for `dialect`, with its parameters in bind order.
  pub fn to_sql_for(&self, dialect: Dialect) -> (String, Vec<Value>) {
    match dialect {
      Dialect::Sqlite => self.to_sql_sqlite(),
      Dialect::Postgres => self.to_sql_postgres(),
    }
  }

  /// [`Self::to_sql_for`] against [`Dialect::CURRENT`].
  pub fn to_sql(&self) -> (String, Vec<Value>) {
    self.to_sql_for(Dialect::CURRENT)
  }

  /// `INSERT [OR REPLACE | OR IGNORE] INTO <target> <rows> [<conflict>] [<returning>]`.
  ///
  /// SQLite spells `or_replace()` / `or_ignore()` as a keyword on the `INSERT`
  /// itself; the explicit clause renders after the rows, like Postgres's.
  fn to_sql_sqlite(&self) -> (String, Vec<Value>) {
    let mut sql = String::new();

    let keyword = match self.conflict_mode {
      ConflictMode::Replace => "INSERT OR REPLACE INTO",
      ConflictMode::Ignore => "INSERT OR IGNORE INTO",
      ConflictMode::None | ConflictMode::Clause(_) => "INSERT INTO",
    };

    sql.push_str(keyword);
    sql.push(' ');
    self.push_target(&mut sql);
    let mut params = BoundParams::new();
    self.push_rows(&mut sql, &mut params, Dialect::Sqlite);

    if let ConflictMode::Clause(clause) = &self.conflict_mode {
      clause.push_sql(&mut sql, &mut params, Dialect::Sqlite);
    }
    self.push_returning(&mut sql);

    (sql, params.into_values())
  }

  /// `INSERT INTO <target> <rows> [<conflict>] [<returning>]`.
  ///
  /// Postgres has no `INSERT OR …` keyword, so every conflict policy renders
  /// as an `ON CONFLICT` clause after the rows.
  fn to_sql_postgres(&self) -> (String, Vec<Value>) {
    let mut sql = String::new();

    sql.push_str("INSERT INTO ");
    self.push_target(&mut sql);
    let mut params = BoundParams::new();
    self.push_rows(&mut sql, &mut params, Dialect::Postgres);

    match &self.conflict_mode {
      ConflictMode::None => {},
      ConflictMode::Ignore => sql.push_str(" ON CONFLICT DO NOTHING"),
      ConflictMode::Replace => {
        push_legacy_postgres_replace(&mut sql, self.active_columns(), &self.conflict_cols);
      },
      ConflictMode::Clause(clause) => clause.push_sql(&mut sql, &mut params, Dialect::Postgres),
    }
    self.push_returning(&mut sql);

    (sql, params.into_values())
  }

  /// Appends the target: `"table"`, or `"database"."table"` when qualified.
  ///
  /// Built from the parts rather than through `TableRef::to_sql_fragment_for`,
  /// because an `INSERT INTO` slot takes neither of the other two things a
  /// `TableRef` can carry. An **alias** is dropped: both engines accept
  /// `INSERT INTO t AS x`, but `OnConflict`'s `Scalar::col(&COL)` renders
  /// `"table"."column"` from the column's own table, which an alias would
  /// unname. A table-valued **function** target is not valid SQL anywhere, and
  /// rendering one here would emit placeholders whose values this slot has
  /// nowhere to return — so its arguments are never rendered either.
  fn push_target(&self, sql: &mut String) {
    if let Some(database) = self.table.database() {
      sql.push_str(&quote_ident(database));
      sql.push('.');
    }
    sql.push_str(&quote_ident(self.table.table()));
  }

  /// Appends ` RETURNING "a", "b"` when any column was projected.
  ///
  /// Rendered last and unqualified, which both engines accept, and it binds
  /// nothing — so it takes no `params` argument.
  fn push_returning(&self, sql: &mut String) {
    append_returning(&self.returning, sql);
  }
}
