//! Typed columns of the two tables these suites build statements against.

use toolu_orm_core::column::{Integer, Text};
use toolu_orm_core::query_column::Column;

pub const MEMORY_ID: Column<Text> = Column::new("memories", "id");
pub const BODY: Column<Text> = Column::new("memories", "body");
pub const WORKSPACE_ID: Column<Text> = Column::new("memories", "workspace_id");
pub const USED_COUNT: Column<Integer> = Column::new("memories", "used_count");
pub const LAST_USED: Column<Text> = Column::new("memories", "last_used");

pub const SYMBOL_ID: Column<Integer> = Column::new("code_symbols", "id");
pub const REPO: Column<Text> = Column::new("code_symbols", "repo");
pub const PATH: Column<Text> = Column::new("code_symbols", "path");
pub const INDEXED_AT: Column<Text> = Column::new("code_symbols", "indexed_at");

/// `strftime('%Y-%m-%dT%H:%M:%fZ', 'now')` with both arguments bound, the
/// database-clock expression issue #108 names.
pub const ISO_FORMAT: &str = "%Y-%m-%dT%H:%M:%fZ";
