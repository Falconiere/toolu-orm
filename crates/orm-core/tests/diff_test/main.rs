//! Tests for schema diffing (migration generation).

mod enum_diffs;
mod fk_diffs;
mod index_diffs;
mod rename_diffs;
mod table_diff_scenarios;
mod table_operation_variants;
mod test_helpers;

pub(crate) use test_helpers::{col, table};
