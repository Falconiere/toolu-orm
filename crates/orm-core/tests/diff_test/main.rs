//! Tests for schema diffing (migration generation).

#[path = "../fixtures/composite_fk_schema.rs"]
pub mod composite_fk_schema;

mod composite_fk_diffs;
mod enum_diffs;
mod fk_diffs;
mod index_diffs;
mod primary_key_diffs;
mod rename_diffs;
mod table_diff_scenarios;
mod table_operation_variants;
mod test_helpers;
mod virtual_table_diffs;
mod virtual_table_fixture;
mod virtual_table_recreate;
mod virtual_table_renames;

pub(crate) use test_helpers::{col, table};
