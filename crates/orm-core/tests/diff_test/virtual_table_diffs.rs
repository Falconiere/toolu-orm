//! Virtual tables in the diff: create, drop and rename are ordinary
//! operations; every in-place change is refused, because SQLite has no
//! `ALTER TABLE` for them.

use toolu_orm_core::column::ColumnType;
use toolu_orm_core::diff::{diff, diff_with_resolver, Operation};
use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::fts5::Fts5Table;
use toolu_orm_core::index::IndexDef;
use toolu_orm_core::rename::RenameResolver;
use toolu_orm_core::schema::SchemaRegistry;
use toolu_orm_core::snapshot::Snapshot;
use toolu_orm_core::table::{TableDef, TableKind};

use super::test_helpers::{col, table};

const NO_INDEXES_REASON: &str = "virtual tables cannot declare indexes";

fn memory_fts() -> TableDef {
  Fts5Table::new("memory_fts")
    .unindexed_column("memory_id", ColumnType::Text)
    .column("body", ColumnType::Text)
    .tokenize("porter")
    .build()
}

fn registry(tables: Vec<TableDef>) -> SchemaRegistry {
  SchemaRegistry::from_tables(tables)
}

fn snapshot_of(tables: Vec<TableDef>) -> Snapshot {
  Snapshot::from_registry(&registry(tables))
}

/// The refusal message, or a description of whatever came back instead — the
/// caller compares it to the expected refusal, so either way the assertion
/// names what happened.
fn refusal(old: &Snapshot, new: &SchemaRegistry) -> String {
  match diff(old, new) {
    Err(DbCoreError::VirtualTableChange { table, reason }) => format!("{table}: {reason}"),
    Err(other) => format!("a different error: {other}"),
    Ok(ops) => format!("no refusal at all, but {ops:?}"),
  }
}

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
  });
  assert_eq!(
    refusal(&Snapshot::empty(), &registry(vec![indexed])),
    format!("memory_fts: {NO_INDEXES_REASON}")
  );
}

struct RenameMemoryFts;

impl RenameResolver for RenameMemoryFts {
  fn resolve_tables(&self, added: &[String], removed: &[String]) -> Vec<(String, String)> {
    if added.iter().any(|a| a == "note_fts") && removed.iter().any(|r| r == "memory_fts") {
      return vec![("memory_fts".to_owned(), "note_fts".to_owned())];
    }
    vec![]
  }

  fn resolve_columns(
    &self,
    _table: &str,
    _added: &[String],
    _removed: &[String],
  ) -> Vec<(String, String)> {
    vec![]
  }
}

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

/// The rename path reaches the same checks as the unrenamed one: a rename is
/// not a way to smuggle an in-place change past them.
#[test]
fn renaming_a_virtual_table_while_changing_it_is_refused() {
  let old = snapshot_of(vec![memory_fts()]);
  let mut retokenized = Fts5Table::new("note_fts")
    .unindexed_column("memory_id", ColumnType::Text)
    .column("body", ColumnType::Text)
    .tokenize("unicode61")
    .build();
  retokenized.indexes.clear();
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
    columns: vec!["body".to_owned()],
    unique: false,
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
