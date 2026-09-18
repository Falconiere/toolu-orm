//! INSERT query builder with conflict handling (ON CONFLICT / OR REPLACE).

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::Scalar;
use toolu_orm_core::query_column::Column;
use toolu_orm_core::value::Value;

use crate::where_clause::cfg_single_backend;

cfg_single_backend! {
  use crate::exec_helpers::impl_execute;
}

// ── ConflictMode ──────────────────────────────────────────────────────────────

enum ConflictMode {
  None,
  Replace,
  Ignore,
}

// ── InsertBuilder ─────────────────────────────────────────────────────────────

pub struct InsertBuilder {
  table: String,
  columns: Vec<String>,
  /// One scalar per column, in the same order; a plain `set` stores a bind.
  values: Vec<Scalar>,
  conflict_mode: ConflictMode,
  conflict_cols: Vec<String>,
}

impl InsertBuilder {
  pub fn new(table: &str) -> Self {
    Self {
      table: table.to_owned(),
      columns: Vec::new(),
      values: Vec::new(),
      conflict_mode: ConflictMode::None,
      conflict_cols: Vec::new(),
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

  pub fn or_replace(mut self) -> Self {
    self.conflict_mode = ConflictMode::Replace;
    self
  }

  pub fn or_ignore(mut self) -> Self {
    self.conflict_mode = ConflictMode::Ignore;
    self
  }

  /// Columns that form the `ON CONFLICT (...)` target for Postgres.
  ///
  /// Ignored for SQLite (`INSERT OR REPLACE` / `INSERT OR IGNORE`).
  /// If unset, the first inserted column is used as the conflict target.
  pub fn conflict_columns(mut self, cols: &[&str]) -> Self {
    self.conflict_cols = cols.iter().map(|c| (*c).to_owned()).collect();
    self
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
      ConflictMode::None => "INSERT INTO",
      ConflictMode::Replace => "INSERT OR REPLACE INTO",
      ConflictMode::Ignore => "INSERT OR IGNORE INTO",
    };

    sql.push_str(keyword);
    sql.push_str(&format!(r#" "{}""#, self.table));
    let params = self.push_columns_and_values(&mut sql, Dialect::Sqlite);

    (sql, params)
  }

  fn to_sql_postgres(&self) -> (String, Vec<Value>) {
    let mut sql = String::new();

    sql.push_str(&format!(r#"INSERT INTO "{}""#, self.table));
    let params = self.push_columns_and_values(&mut sql, Dialect::Postgres);

    match self.conflict_mode {
      ConflictMode::None => {},
      ConflictMode::Ignore => {
        sql.push_str(" ON CONFLICT DO NOTHING");
      },
      ConflictMode::Replace => {
        let conflict_target = if !self.conflict_cols.is_empty() {
          self.conflict_cols.clone()
        } else if let Some(first) = self.columns.first() {
          vec![first.clone()]
        } else {
          Vec::new()
        };

        let conflict_cols_sql: Vec<String> = conflict_target
          .iter()
          .map(|c| format!(r#""{c}""#))
          .collect();

        let update_cols: Vec<String> = self
          .columns
          .iter()
          .filter(|c| !conflict_target.iter().any(|t| t == *c))
          .map(|c| format!(r#""{c}" = EXCLUDED."{c}""#))
          .collect();

        sql.push_str(&format!(
          " ON CONFLICT ({}) DO ",
          conflict_cols_sql.join(", ")
        ));

        if update_cols.is_empty() {
          sql.push_str("NOTHING");
        } else {
          sql.push_str(&format!("UPDATE SET {}", update_cols.join(", ")));
        }
      },
    }

    (sql, params)
  }
}

cfg_single_backend! {
  impl_execute!(InsertBuilder, "INSERT");
}
