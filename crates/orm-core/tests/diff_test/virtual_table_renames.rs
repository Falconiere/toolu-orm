//! Renaming a virtual table is an ordinary `RenameTable`, and it is not a way
//! to smuggle an in-place change past the refusals.

use toolu_orm_core::column::ColumnType;
use toolu_orm_core::diff::{diff_with_resolver, Operation};
use toolu_orm_core::fts5::Fts5Table;
use toolu_orm_core::index::IndexDef;

use super::test_helpers::{col, table};
use super::virtual_table_fixture::{
  memory_fts, registry, snapshot_of, RenameMemoryFts, NO_INDEXES_REASON,
};

#[test]
fn renaming_a_virtual_table_renames_it_without_recreating() {
  let old = snapshot_of(vec![memory_fts()]);
  let mut renamed = memory_fts();
  renamed.name = "note_fts".to_owned();
  let ops = diff_with_resolver(&old, &registry(vec![renamed]), &RenameMemoryFts)
    .expect("diff should succeed");
  assert_eq!(
    ops,
    vec![Operation::RenameTable {
      old: "memory_fts".to_owned(),
      new: "note_fts".to_owned()
    }]
  );
}

/// The rename path reaches the same checks as the unrenamed one.
#[test]
fn renaming_a_virtual_table_while_changing_it_is_refused() {
  let old = snapshot_of(vec![memory_fts()]);
  let retokenized = Fts5Table::new("note_fts")
    .unindexed_column("memory_id", ColumnType::Text)
    .column("body", ColumnType::Text)
    .tokenize("unicode61")
    .build();
  let error = diff_with_resolver(&old, &registry(vec![retokenized]), &RenameMemoryFts)
    .expect_err("expected the change to be refused");
  assert!(
    error
      .to_string()
      .contains("note_fts\" in place: its module arguments changed"),
    "unexpected error: {error}"
  );
}

#[test]
fn renaming_a_virtual_table_onto_an_index_is_refused() {
  let old = snapshot_of(vec![memory_fts()]);
  let mut indexed = memory_fts();
  indexed.name = "note_fts".to_owned();
  indexed.indexes.push(IndexDef {
    name: "idx_note_fts_body".to_owned(),
    columns: vec!["body".into()],
    unique: false,
    where_clause: None,
  });
  let error = diff_with_resolver(&old, &registry(vec![indexed]), &RenameMemoryFts)
    .expect_err("expected the change to be refused");
  assert!(
    error.to_string().contains(NO_INDEXES_REASON),
    "unexpected error: {error}"
  );
}

/// Renaming an ordinary table onto a virtual one is still a kind change.
#[test]
fn renaming_an_ordinary_table_into_a_virtual_one_is_refused() {
  let ordinary = table(
    "memory_fts",
    vec![col("body", ColumnType::Text, false, false)],
  );
  let old = snapshot_of(vec![ordinary]);
  let mut renamed = memory_fts();
  renamed.name = "note_fts".to_owned();
  let error = diff_with_resolver(&old, &registry(vec![renamed]), &RenameMemoryFts)
    .expect_err("expected the change to be refused");
  assert!(
    error
      .to_string()
      .contains("it became a virtual table using fts5"),
    "unexpected error: {error}"
  );
}
