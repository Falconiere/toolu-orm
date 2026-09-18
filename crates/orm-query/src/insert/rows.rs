//! Where an `INSERT`'s rows come from: a `VALUES` list, or a whole `SELECT`.

use toolu_orm_core::alias::quote_ident;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::SelectSource;
use toolu_orm_core::query_column::ColumnRef;
use toolu_orm_core::value::Value;

use super::conflict::ConflictMode;
use super::InsertBuilder;

/// The alias the SQLite guard gives the derived table it wraps a source in.
///
/// Prefixed like `"toolu_count"` in the counted-select rendering, so it cannot
/// collide with a caller's own name by accident.
const GUARD_ALIAS: &str = "toolu_insert_source";

/// A complete `SELECT` supplying one target row per result row, with the
/// target columns it fills.
pub(super) struct SelectRows {
  /// Target column names, in the order the source projects them.
  pub(super) columns: Vec<String>,
  pub(super) source: Box<dyn SelectSource>,
}

impl InsertBuilder {
  /// `INSERT INTO "t" ("a", "b") <select>` — a set-based copy whose rows are
  /// never decoded into Rust.
  ///
  /// `columns` names the **target** columns; the source must project the same
  /// number in the same order, and may project a literal for one the source
  /// table does not have. Calling this makes the `SELECT` the row source, so
  /// any value recorded by [`set`](Self::set) is dropped and a later `set` goes
  /// on being ignored — both are still held on the builder, and neither is
  /// rendered. An empty `columns` renders `INSERT INTO "t" <select>`,
  /// matching positionally rather than emitting an empty `()`. `returning`
  /// projects one row per *inserted* row here, so read it with `fetch_all`.
  #[must_use]
  pub fn select(self, columns: &[&dyn ColumnRef], source: impl SelectSource + 'static) -> Self {
    let names: Vec<String> = columns.iter().map(|c| c.name().to_owned()).collect();
    self.select_named(names, source)
  }

  /// [`Self::select`] with the **target columns** spelled as plain strings —
  /// `_raw` qualifies the columns, not the statement, exactly as it does in
  /// `SelectBuilder::columns_raw`.
  #[must_use]
  pub fn select_raw(self, columns: &[&str], source: impl SelectSource + 'static) -> Self {
    let names: Vec<String> = columns.iter().map(|c| (*c).to_owned()).collect();
    self.select_named(names, source)
  }

  fn select_named(mut self, columns: Vec<String>, source: impl SelectSource + 'static) -> Self {
    self.select = Some(SelectRows {
      columns,
      source: Box::new(source),
    });
    self
  }

  /// The column list the statement actually renders: the `SELECT`'s targets
  /// when there is one, else the columns `set*` recorded.
  pub(super) fn active_columns(&self) -> &[String] {
    match &self.select {
      Some(rows) => &rows.columns,
      None => &self.columns,
    }
  }

  /// Appends ` ("a", "b")`, or nothing when no column was named.
  fn push_column_list(&self, sql: &mut String) {
    let columns = self.active_columns();
    if columns.is_empty() {
      return;
    }
    let rendered: Vec<String> = columns.iter().map(|c| quote_ident(c)).collect();
    sql.push_str(&format!(" ({})", rendered.join(", ")));
  }

  /// Appends the column list and the rows, returning the values they bind in
  /// the order their placeholders were written.
  pub(super) fn push_rows(&self, sql: &mut String, dialect: Dialect) -> Vec<Value> {
    self.push_column_list(sql);
    match &self.select {
      Some(rows) => self.push_select_rows(sql, rows, dialect),
      None => self.push_values(sql, dialect),
    }
  }

  /// Appends ` VALUES (<a>, <b>)`.
  fn push_values(&self, sql: &mut String, dialect: Dialect) -> Vec<Value> {
    sql.push_str(" VALUES (");

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

  /// Appends ` <select>`, numbered from `1`: the target and the column list
  /// are written before it and neither binds a value.
  ///
  /// The statement returned by `to_select_sql_for` is complete and
  /// unparenthesised — `WITH` prefix and `UNION` arms included — so it is
  /// appended whole. Whatever renders after it (`ON CONFLICT`, `RETURNING`)
  /// continues from the live `params.len()`, which is how a source's binds end
  /// up ahead of a conflict clause's with no arithmetic anywhere.
  fn push_select_rows(&self, sql: &mut String, rows: &SelectRows, dialect: Dialect) -> Vec<Value> {
    sql.push(' ');
    if self.needs_sqlite_guard(dialect) {
      return push_guarded(sql, rows, dialect);
    }
    let (select_sql, params) = rows.source.to_select_sql_for(1, dialect);
    sql.push_str(&select_sql);
    params
  }

  /// Whether SQLite would misread the `ON` of a following `ON CONFLICT` as a
  /// join's `ON`.
  ///
  /// Measured on SQLite 3.51.0: `INSERT INTO "u" ("a") SELECT "a" FROM "s" ON
  /// CONFLICT ("a") DO UPDATE SET …` is `Parse error: near "DO"`. It applies
  /// only to the explicit clause — `or_ignore()` / `or_replace()` are `INSERT
  /// OR …` keywords on SQLite, with no trailing `ON` to confuse. Postgres
  /// parses every one of those shapes unwrapped, so it is never guarded.
  fn needs_sqlite_guard(&self, dialect: Dialect) -> bool {
    matches!(dialect, Dialect::Sqlite) && matches!(self.conflict_mode, ConflictMode::Clause(_))
  }
}

/// `SELECT * FROM (<source>) AS "toolu_insert_source" WHERE true`.
///
/// SQLite's own documented workaround for the ambiguity above is to give the
/// source a `WHERE`. This builder cannot add one to an opaque [`SelectSource`]
/// that may already carry `GROUP BY`, `ORDER BY`, `LIMIT` or a `UNION`, so it
/// supplies the clause on a derived table instead. Parenthesising the source
/// directly is not an alternative: SQLite rejects `INSERT INTO "t" ("a")
/// (SELECT …)` outright.
///
/// The inner statement still numbers from `1`: the wrapper binds nothing.
fn push_guarded(sql: &mut String, rows: &SelectRows, dialect: Dialect) -> Vec<Value> {
  let (select_sql, params) = rows.source.to_select_sql_for(1, dialect);
  sql.push_str(&format!(
    "SELECT * FROM ({select_sql}) AS {} WHERE true",
    quote_ident(GUARD_ALIAS)
  ));
  params
}
