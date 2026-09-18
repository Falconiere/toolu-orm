//! The columns and helpers the reusable-binding assertions share.

use toolu_orm_core::column::{Integer, Text};
use toolu_orm_core::query_column::Column;
use toolu_orm_core::value::Value;

pub const SRC_ID: Column<Text> = Column::new("edges", "src_id");
pub const DST_ID: Column<Text> = Column::new("edges", "dst_id");
pub const REL: Column<Text> = Column::new("edges", "rel");
pub const WEIGHT: Column<Integer> = Column::new("edges", "weight");

pub fn text(value: &str) -> Value {
  Value::Text(value.to_owned())
}

/// The placeholder indices a SQLite rendering names, in written order.
///
/// Reading them back out of the SQL is what proves reuse: the same index
/// appearing twice is the whole feature, and a gap would mean a value was
/// bound without being referenced.
pub fn sqlite_indices(sql: &str) -> Vec<usize> {
  let mut indices = Vec::new();
  let mut chars = sql.chars().peekable();
  while let Some(ch) = chars.next() {
    if ch != '?' {
      continue;
    }
    let mut digits = String::new();
    while chars.peek().is_some_and(char::is_ascii_digit) {
      if let Some(digit) = chars.next() {
        digits.push(digit);
      }
    }
    if let Ok(index) = digits.parse::<usize>() {
      indices.push(index);
    }
  }
  indices
}
