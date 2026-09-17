//! The Operation enum and ColumnChange for schema migrations.

use crate::column::{ColumnDef, ColumnType};
use crate::fts5::Fts5Sync;
use crate::index::IndexDef;
use crate::snapshot::ForeignKeyDef;
use crate::table::TableDef;

/// One column-level difference between two schema versions.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ColumnChange {
  Type {
    column: String,
    old: ColumnType,
    new: ColumnType,
  },
  Default {
    column: String,
    old: Option<String>,
    new: Option<String>,
  },
  Nullable {
    column: String,
    old: bool,
    new: bool,
  },
  Unique {
    column: String,
    old: bool,
    new: bool,
  },
  /// Column-level `primary_key` flag flipped; SQLite must recreate the table.
  PrimaryKey {
    column: String,
    old: bool,
    new: bool,
  },
  /// `autoincrement` flag flipped; SQLite must recreate the table.
  Autoincrement {
    column: String,
    old: bool,
    new: bool,
  },
  /// Table-level composite primary key column list changed.
  CompositePrimaryKey { old: Vec<String>, new: Vec<String> },
}

/// One migration step, before any dialect renders it.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Operation {
  CreateEnum {
    name: String,
    variants: Vec<String>,
  },
  AlterEnum {
    name: String,
    added: Vec<String>,
    removed: Vec<String>,
  },
  DropEnum {
    name: String,
  },
  CreateTable {
    table: TableDef,
  },
  DropTable {
    name: String,
  },
  RenameTable {
    old: String,
    new: String,
  },
  AddColumn {
    table: String,
    column: ColumnDef,
  },
  DropColumn {
    table: String,
    column: String,
  },
  RenameColumn {
    table: String,
    old: String,
    new: String,
  },
  AlterColumn {
    table: String,
    changes: Vec<ColumnChange>,
    table_def: TableDef,
  },
  CreateIndex {
    table: String,
    index: IndexDef,
  },
  DropIndex {
    name: String,
  },
  AddForeignKey {
    table: String,
    fk: ForeignKeyDef,
  },
  DropForeignKey {
    table: String,
    name: String,
  },
  AddCheckConstraint {
    table: String,
    name: String,
    expr: String,
  },
  DropCheckConstraint {
    table: String,
    name: String,
  },
  /// Drop + recreate an FTS5 virtual table and rebuild it from its external
  /// content table (`INSERT INTO fts(fts) VALUES('rebuild')`).
  ///
  /// When the table declares [`Fts5Sync`], the rebuild moves into
  /// [`Self::CreateFts5SyncTriggers`] so the index is filled after its triggers
  /// exist rather than before.
  RecreateFts5FromContent {
    table: TableDef,
  },
  /// Drop the three generated synchronization triggers for an FTS5 table.
  /// Ordered before anything that drops or rebuilds either table.
  DropFts5SyncTriggers {
    table: String,
  },
  /// Create the three generated synchronization triggers for an FTS5 table and
  /// rebuild its index. Ordered after both tables exist.
  CreateFts5SyncTriggers {
    table: String,
    sync: Fts5Sync,
  },
}
