//! Builder that turns columns and options into an FTS5 [`TableDef`].

use crate::column::{ColumnDef, ColumnType};
use crate::table::{TableDef, TableKind};

use super::options::Fts5Options;

/// The SQLite module name for full-text search tables.
pub const FTS5_MODULE: &str = "fts5";

/// Assembles `CREATE VIRTUAL TABLE … USING fts5(…)` without hand-writing the
/// module arguments.
///
/// ```
/// use toolu_orm_core::column::ColumnType;
/// use toolu_orm_core::fts5::Fts5Table;
///
/// let table = Fts5Table::new("memory_fts")
///   .unindexed_column("memory_id", ColumnType::Text)
///   .column("body", ColumnType::Text)
///   .tokenize("porter unicode61")
///   .build();
///
/// assert_eq!(table.kind.module(), Some("fts5"));
/// assert_eq!(table.kind.args()[0], "\"memory_id\" UNINDEXED");
/// ```
#[derive(Debug, Clone, Default)]
pub struct Fts5Table {
  name: String,
  columns: Vec<ColumnDef>,
  options: Fts5Options,
}

impl Fts5Table {
  #[must_use]
  pub fn new(name: impl Into<String>) -> Self {
    Self {
      name: name.into(),
      columns: Vec::new(),
      options: Fts5Options::default(),
    }
  }

  /// A searchable column. FTS5 stores every column as text; `column_type` is
  /// kept as metadata for the generated typed columns and is never rendered
  /// into the DDL.
  #[must_use]
  pub fn column(self, name: impl Into<String>, column_type: ColumnType) -> Self {
    self.push_column(name.into(), column_type, false)
  }

  /// A column stored but not indexed (`UNINDEXED`), so it is returned by
  /// queries but never matched by `MATCH`.
  #[must_use]
  pub fn unindexed_column(self, name: impl Into<String>, column_type: ColumnType) -> Self {
    self.push_column(name.into(), column_type, true)
  }

  /// The tokenizer chain, e.g. `porter unicode61 remove_diacritics 2`.
  #[must_use]
  pub fn tokenize(mut self, spec: impl Into<String>) -> Self {
    self.options.tokenize = Some(spec.into());
    self
  }

  /// Prefix index sizes, e.g. `2 3`.
  #[must_use]
  pub fn prefix(mut self, spec: impl Into<String>) -> Self {
    self.options.prefix = Some(spec.into());
    self
  }

  /// External content table; the empty string makes the table contentless.
  #[must_use]
  pub fn content(mut self, table: impl Into<String>) -> Self {
    self.options.content = Some(table.into());
    self
  }

  /// The rowid column of the external content table.
  #[must_use]
  pub fn content_rowid(mut self, column: impl Into<String>) -> Self {
    self.options.content_rowid = Some(column.into());
    self
  }

  /// `columnsize = 0` drops the per-column size index.
  #[must_use]
  pub fn columnsize(mut self, value: u8) -> Self {
    self.options.columnsize = Some(value);
    self
  }

  /// Detail level: `full`, `column`, or `none`.
  #[must_use]
  pub fn detail(mut self, value: impl Into<String>) -> Self {
    self.options.detail = Some(value.into());
    self
  }

  /// The finished definition, with the module arguments rendered into
  /// [`TableKind::Virtual`].
  #[must_use]
  pub fn build(self) -> TableDef {
    let mut args: Vec<String> = self.columns.iter().map(column_arg).collect();
    args.extend(self.options.render());
    TableDef {
      name: self.name,
      columns: self.columns,
      indexes: Vec::new(),
      primary_key: Vec::new(),
      strict: false,
      kind: TableKind::virtual_table(FTS5_MODULE, args),
    }
  }

  fn push_column(mut self, name: String, column_type: ColumnType, unindexed: bool) -> Self {
    self.columns.push(ColumnDef {
      name,
      column_type,
      primary_key: false,
      not_null: false,
      default: None,
      unique: false,
      references: None,
      on_delete: None,
      on_update: None,
      check: None,
      unindexed,
      autoincrement: false,
    });
    self
  }
}

fn column_arg(column: &ColumnDef) -> String {
  if column.unindexed {
    format!("\"{}\" UNINDEXED", column.name)
  } else {
    format!("\"{}\"", column.name)
  }
}
