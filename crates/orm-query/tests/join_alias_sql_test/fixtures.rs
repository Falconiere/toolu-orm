//! Typed columns and the one builder the parameter-order assertions share.

use toolu_orm_core::column::{Integer, Text};
use toolu_orm_core::query_column::{Column, CommonOps};
use toolu_orm_query::select::SelectBuilder;

pub const C_ID: Column<Text> = Column::new("code_symbols", "id");
pub const C_REPO: Column<Text> = Column::new("code_symbols", "repo");
pub const C_PATH: Column<Text> = Column::new("code_symbols", "path");
pub const C_KIND: Column<Text> = Column::new("code_symbols", "kind");
pub const F_ID: Column<Text> = Column::new("code_feedback", "id");
pub const F_REPO: Column<Text> = Column::new("code_feedback", "repo");
pub const F_PATH: Column<Text> = Column::new("code_feedback", "path");
pub const F_LIVE: Column<Integer> = Column::new("code_feedback", "live");

/// The builder every parameter-order assertion below renders.
pub fn ordered_builder() -> SelectBuilder {
  SelectBuilder::new("code_symbols")
    .columns_qualified(&[&C_ID])
    .left_join("code_feedback", F_REPO.equals(&C_REPO).and(F_LIVE.eq(1)))
    .filter(C_KIND.eq("note"))
    .limit(5)
}
