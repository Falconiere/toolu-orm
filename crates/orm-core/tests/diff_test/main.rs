//! Tests for schema diffing (migration generation).

mod enum_diffs;
mod fk_diffs;
mod index_diffs;
mod rename_diffs;
mod table_diff_scenarios;
mod table_operation_variants;
mod test_helpers;
mod virtual_table_diffs;
mod virtual_table_fixture;
mod virtual_table_renames;

pub(crate) use test_helpers::{col, table};
