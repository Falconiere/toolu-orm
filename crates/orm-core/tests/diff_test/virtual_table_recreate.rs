//! External-content FTS5: eligible in-place changes become
//! `RecreateFts5FromContent`; missing content / columns still refuse.

use toolu_orm_core::column::ColumnType;
use toolu_orm_core::diff::{diff, Operation};
use toolu_orm_core::fts5::Fts5Table;
use toolu_orm_core::table::TableDef;

use super::test_helpers::{col, table};
use super::virtual_table_fixture::{refusal, registry, snapshot_of};

fn memories() -> TableDef {
  table(
    "memories",
    vec![
      col("id", ColumnType::Text, true, true),
      col("body", ColumnType::Text, false, false),
    ],
  )
}

fn memory_fts_external(tokenize: &str) -> TableDef {
  Fts5Table::new("memory_fts")
    .column("body", ColumnType::Text)
    .tokenize(tokenize)
    .content("memories")
    .content_rowid("id")
    .build()
}

#[test]
fn changing_tokenizer_with_external_content_recreates() {
  let old = snapshot_of(vec![memories(), memory_fts_external("porter")]);
  let ops = diff(
    &old,
    &registry(vec![memories(), memory_fts_external("unicode61")]),
  )
  .expect("eligible recreate should succeed");
  assert_eq!(
    ops,
    vec![Operation::RecreateFts5FromContent {
      table: memory_fts_external("unicode61")
    }]
  );
}

#[test]
fn changing_columns_with_external_content_recreates_when_content_covers_them() {
  let content = table(
    "memories",
    vec![
      col("id", ColumnType::Text, true, true),
      col("body", ColumnType::Text, false, false),
      col("tags", ColumnType::Text, false, false),
    ],
  );
  let old = snapshot_of(vec![content.clone(), memory_fts_external("porter")]);
  let widened = Fts5Table::new("memory_fts")
    .column("body", ColumnType::Text)
    .column("tags", ColumnType::Text)
    .tokenize("porter")
    .content("memories")
    .content_rowid("id")
    .build();
  let ops = diff(&old, &registry(vec![content, widened.clone()])).expect("recreate");
  assert_eq!(
    ops,
    vec![Operation::RecreateFts5FromContent { table: widened }]
  );
}

#[test]
fn tokenizer_change_without_content_is_still_refused() {
  let standalone = Fts5Table::new("memory_fts")
    .column("body", ColumnType::Text)
    .tokenize("porter")
    .build();
  let retokenized = Fts5Table::new("memory_fts")
    .column("body", ColumnType::Text)
    .tokenize("unicode61")
    .build();
  let old = snapshot_of(vec![standalone]);
  assert_eq!(
    refusal(&old, &registry(vec![retokenized])),
    "memory_fts: its module arguments changed"
  );
}

#[test]
fn missing_content_table_is_refused() {
  let old = snapshot_of(vec![memories(), memory_fts_external("porter")]);
  assert_eq!(
    refusal(&old, &registry(vec![memory_fts_external("unicode61")])),
    "memory_fts: its content table \"memories\" is not in the schema"
  );
}

#[test]
fn content_table_missing_fts_column_is_refused() {
  let old = snapshot_of(vec![memories(), memory_fts_external("porter")]);
  let widened = Fts5Table::new("memory_fts")
    .column("body", ColumnType::Text)
    .column("tags", ColumnType::Text)
    .tokenize("porter")
    .content("memories")
    .content_rowid("id")
    .build();
  assert_eq!(
    refusal(&old, &registry(vec![memories(), widened])),
    "memory_fts: its content table \"memories\" is missing column \"tags\""
  );
}

#[test]
fn refuse_display_mentions_external_content_rebuild() {
  let standalone = Fts5Table::new("memory_fts")
    .column("body", ColumnType::Text)
    .tokenize("porter")
    .build();
  let retokenized = Fts5Table::new("memory_fts")
    .column("body", ColumnType::Text)
    .tokenize("unicode61")
    .build();
  let old = snapshot_of(vec![standalone]);
  let err = diff(&old, &registry(vec![retokenized])).expect_err("refuse");
  let message = err.to_string();
  assert!(
    message.contains("memory_fts")
      && message.contains("module arguments")
      && message.contains("content =")
      && message.contains("rebuild"),
    "unhelpful error: {message}"
  );
}
