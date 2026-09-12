//! `desc(column)` inside `#[index]` / `#[unique_index]`.

use toolu_orm_core::column::{Text, Timestamp, Uuid};
use toolu_orm_core::index::IndexColumn;
use toolu_orm_core::table::TableSchema;
use toolu_orm_macros::table;

#[table(name = "eval_runs")]
#[index("idx_eval_runs_at", desc(at))]
#[index("idx_eval_runs_mixed", id, desc(at))]
#[unique_index("uq_eval_runs_started", desc(started_at))]
#[index("idx_eval_runs_partial", desc(at), where = "status = 'done'")]
pub struct EvalRuns {
  #[column(primary_key)]
  pub id: Uuid,
  pub at: Timestamp,
  pub started_at: Timestamp,
  pub note: Text,
  pub status: Text,
}

#[test]
fn desc_columns_reach_table_def() -> Result<(), Box<dyn std::error::Error>> {
  let def = EvalRuns::table_def();
  assert_eq!(def.indexes.len(), 4);

  let at = def
    .indexes
    .iter()
    .find(|i| i.name == "idx_eval_runs_at")
    .ok_or("missing idx_eval_runs_at")?;
  assert_eq!(at.columns, vec![IndexColumn::desc("at")]);
  assert!(!at.unique);

  let mixed = def
    .indexes
    .iter()
    .find(|i| i.name == "idx_eval_runs_mixed")
    .ok_or("missing idx_eval_runs_mixed")?;
  assert_eq!(
    mixed.columns,
    vec![IndexColumn::new("id"), IndexColumn::desc("at")]
  );

  let unique = def
    .indexes
    .iter()
    .find(|i| i.name == "uq_eval_runs_started")
    .ok_or("missing uq_eval_runs_started")?;
  assert_eq!(unique.columns, vec![IndexColumn::desc("started_at")]);
  assert!(unique.unique);

  let partial = def
    .indexes
    .iter()
    .find(|i| i.name == "idx_eval_runs_partial")
    .ok_or("missing idx_eval_runs_partial")?;
  assert_eq!(partial.columns, vec![IndexColumn::desc("at")]);
  assert_eq!(partial.where_clause.as_deref(), Some("status = 'done'"));
  Ok(())
}
