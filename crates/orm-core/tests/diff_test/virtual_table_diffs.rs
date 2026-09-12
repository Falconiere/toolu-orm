//! Virtual tables in the diff: create and drop are ordinary operations; every
//! in-place change is refused, because SQLite has no `ALTER TABLE` for them.

use toolu_orm_core::column::ColumnType;
use toolu_orm_core::diff::{diff, Operation};
use toolu_orm_core::fts5::Fts5Table;
use toolu_orm_core::index::IndexDef;
use toolu_orm_core::snapshot::Snapshot;
use toolu_orm_core::table::TableKind;

use super::test_helpers::{col, table};
use super::virtual_table_fixture::{memory_fts, refusal, registry, snapshot_of, NO_INDEXES_REASON};

#[test]
fn adding_a_virtual_table_creates_it() {
  let ops = diff(&Snapshot::empty(), &registry(vec![memory_fts()])).expect("diff should succeed");
  assert_eq!(
    ops,
    vec![Operation::CreateTable {
      table: memory_fts()
    }]
  );
}

#[test]
fn an_unchanged_virtual_table_produces_no_operations() {
  let old = snapshot_of(vec![memory_fts()]);
  let ops = diff(&old, &registry(vec![memory_fts()])).expect("diff should succeed");
  assert_eq!(ops, vec![]);
}

#[test]
fn removing_a_virtual_table_drops_it() {
  let old = snapshot_of(vec![memory_fts()]);
  let ops = diff(&old, &registry(vec![])).expect("diff should succeed");
  assert_eq!(
    ops,
    vec![Operation::DropTable {
      name: "memory_fts".to_owned()
    }]
  );
}

#[test]
fn a_virtual_table_next_to_ordinary_tables_does_not_disturb_them() {
  let memories = table("memories", vec![col("id", ColumnType::Text, true, true)]);
  let old = snapshot_of(vec![memories.clone()]);
  let ops = diff(&old, &registry(vec![memories, memory_fts()])).expect("diff should succeed");
  assert_eq!(
    ops,
    vec![Operation::CreateTable {
      table: memory_fts()
    }]
  );
}

#[test]
fn adding_a_column_is_refused() {
  let old = snapshot_of(vec![memory_fts()]);
  let widened = Fts5Table::new("memory_fts")
    .unindexed_column("memory_id", ColumnType::Text)
    .column("body", ColumnType::Text)
    .column("tags", ColumnType::Text)
    .tokenize("porter")
    .build();
  assert_eq!(
    refusal(&old, &registry(vec![widened])),
    "memory_fts: its columns changed"
  );
}

#[test]
fn flipping_a_column_to_unindexed_is_refused() {
  let old = snapshot_of(vec![memory_fts()]);
  let flipped = Fts5Table::new("memory_fts")
    .unindexed_column("memory_id", ColumnType::Text)
    .unindexed_column("body", ColumnType::Text)
    .tokenize("porter")
    .build();
  assert_eq!(
    refusal(&old, &registry(vec![flipped])),
    "memory_fts: its columns changed"
  );
}

#[test]
fn changing_the_tokenizer_is_refused() {
  let old = snapshot_of(vec![memory_fts()]);
  let retokenized = Fts5Table::new("memory_fts")
    .unindexed_column("memory_id", ColumnType::Text)
    .column("body", ColumnType::Text)
    .tokenize("unicode61")
    .build();
  assert_eq!(
    refusal(&old, &registry(vec![retokenized])),
    "memory_fts: its module arguments changed"
  );
}

#[test]
fn switching_modules_is_refused_by_name() {
  let old = snapshot_of(vec![memory_fts()]);
  let mut moved = memory_fts();
  moved.kind = TableKind::virtual_table("vec0", moved.kind.args().to_vec());
  assert_eq!(
    refusal(&old, &registry(vec![moved])),
    "memory_fts: its module changed from fts5 to vec0"
  );
}

#[test]
fn turning_a_virtual_table_into_an_ordinary_one_is_refused() {
  let old = snapshot_of(vec![memory_fts()]);
  let ordinary = table(
    "memory_fts",
    vec![col("body", ColumnType::Text, false, false)],
  );
  assert_eq!(
    refusal(&old, &registry(vec![ordinary])),
    "memory_fts: it is no longer a virtual table using fts5"
  );
}

#[test]
fn turning_an_ordinary_table_into_a_virtual_one_is_refused() {
  let ordinary = table(
    "memory_fts",
    vec![col("body", ColumnType::Text, false, false)],
  );
  let old = snapshot_of(vec![ordinary]);
  assert_eq!(
    refusal(&old, &registry(vec![memory_fts()])),
    "memory_fts: it became a virtual table using fts5"
  );
}

#[test]
fn declaring_an_index_on_a_new_virtual_table_is_refused() {
  let mut indexed = memory_fts();
  indexed.indexes.push(IndexDef {
    name: "idx_memory_fts_body".to_owned(),
    columns: vec!["body".to_owned()],
    unique: false,
    where_clause: None,
  });
  assert_eq!(
    refusal(&Snapshot::empty(), &registry(vec![indexed])),
    format!("memory_fts: {NO_INDEXES_REASON}")
  );
}
