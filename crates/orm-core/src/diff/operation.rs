//! 16-variant Operation enum and ColumnChange for schema migrations.

use crate::column::{ColumnDef, ColumnType};
use crate::index::IndexDef;
use crate::snapshot::ForeignKeyDef;
use crate::table::TableDef;

#[derive(Debug, Clone, PartialEq)]
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
}

#[derive(Debug, Clone, PartialEq)]
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
}
