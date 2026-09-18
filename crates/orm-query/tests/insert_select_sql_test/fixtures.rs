//! The tables issue #114's copy names, and the builders more than one
//! assertion shares.

use toolu_orm_core::alias::TableRef;
use toolu_orm_core::column::{Integer, Text};
use toolu_orm_core::query_column::Column;
use toolu_orm_core::value::Value;
use toolu_orm_query::select::SelectBuilder;

pub const REPO: Column<Text> = Column::new("indexed_files", "repo");
pub const PATH: Column<Text> = Column::new("indexed_files", "path");
pub const BLOB_OID: Column<Text> = Column::new("indexed_files", "blob_oid");
pub const INDEXED_AT: Column<Integer> = Column::new("indexed_files", "indexed_at");
pub const RANK: Column<Integer> = Column::new("indexed_files", "rank");

/// The target: `indexed_files` of the `main` database.
pub fn target() -> TableRef {
  TableRef::new("indexed_files").in_database("main")
}

/// The source: the same table in the ATTACHed `old` database.
pub fn source() -> TableRef {
  TableRef::new("indexed_files").in_database("old")
}

/// `SELECT "repo", "path", "blob_oid", "indexed_at" FROM "old"."indexed_files"`
/// — the projection that binds nothing.
pub fn plain_projection() -> SelectBuilder {
  SelectBuilder::from_table(source()).columns_raw(&["repo", "path", "blob_oid", "indexed_at"])
}

pub fn text(value: &str) -> Value {
  Value::Text(value.to_owned())
}

/// Every placeholder index the rendered SQL mentions, in order of appearance.
///
/// `?12` and `$12` are one index, not `1` then `2`, so the digits are consumed
/// greedily — the assertion would otherwise pass on a statement it should
/// reject.
pub fn placeholder_indices(sql: &str) -> Vec<usize> {
  let mut found: Vec<usize> = Vec::new();
  let bytes: Vec<char> = sql.chars().collect();
  let mut at = 0;
  while at < bytes.len() {
    let marker = bytes.get(at).copied();
    if marker != Some('?') && marker != Some('$') {
      at += 1;
      continue;
    }
    let mut digits = String::new();
    let mut scan = at + 1;
    while let Some(ch) = bytes.get(scan).copied().filter(char::is_ascii_digit) {
      digits.push(ch);
      scan += 1;
    }
    if let Ok(index) = digits.parse::<usize>() {
      found.push(index);
    }
    at = scan.max(at + 1);
  }
  found
}
