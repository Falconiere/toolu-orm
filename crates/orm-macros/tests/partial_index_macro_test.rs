//! `#[index(..., where = "...")]` and `#[unique_index(..., where = "...")]`
//! land on `IndexDef.where_clause` verbatim.

use toolu_orm_core::column::{Text, Timestamp, Uuid};
use toolu_orm_core::index::IndexColumn;
use toolu_orm_core::table::TableSchema;
use toolu_orm_macros::table;

#[table(name = "memories")]
#[index("idx_memories_repo", repo, where = "deleted_at IS NULL")]
#[unique_index("idx_memories_kind", kind, where = "deleted_at IS NULL")]
#[index("idx_memories_plain", repo)]
pub struct Memories {
  #[column(primary_key)]
  pub id: Uuid,
  pub repo: Text,
  pub kind: Text,
  pub deleted_at: Timestamp,
}

#[test]
fn index_attr_parses_where_predicate() -> Result<(), Box<dyn std::error::Error>> {
  let def = Memories::table_def();
  assert_eq!(def.indexes.len(), 3);

  let partial = def
    .indexes
    .iter()
    .find(|i| i.name == "idx_memories_repo")
    .ok_or("missing idx_memories_repo")?;
  assert_eq!(partial.columns, vec![IndexColumn::new("repo")]);
  assert!(!partial.unique);
  assert_eq!(partial.where_clause.as_deref(), Some("deleted_at IS NULL"));

  let unique = def
    .indexes
    .iter()
    .find(|i| i.name == "idx_memories_kind")
    .ok_or("missing idx_memories_kind")?;
  assert_eq!(unique.columns, vec![IndexColumn::new("kind")]);
  assert!(unique.unique);
  assert_eq!(unique.where_clause.as_deref(), Some("deleted_at IS NULL"));

  let plain = def
    .indexes
    .iter()
    .find(|i| i.name == "idx_memories_plain")
    .ok_or("missing idx_memories_plain")?;
  assert_eq!(plain.where_clause, None);
  Ok(())
}
