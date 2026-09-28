//! `#[table]` registries for the composite foreign-key tests, in the shape of
//! issue #264: every row points at an entity inside its own project.
//!
//! - `project_work_items` references `project_catalog` per column and itself through
//!   `(parent_work_item_id, project_id)` with no action; its unique
//!   `(id, project_id)` index is what every composite key here targets.
//! - `project_evidence` references work items through
//!   `(work_item_id, project_id)`, cascading deletes — declared in v2 only,
//!   so v1 → v2 adds a composite key to an existing table.
//!
//! The per-column parent is `project_catalog`, not `projects`: tables are
//! created in name order, and Postgres resolves an inline column
//! `REFERENCES` when the child table is created, so the parent must sort
//! first. Table-level keys are attached after every table and index exist.
//!
//! Wired into the `composite_fk_*` binaries with `#[path]`.

use toolu_orm_core::schema::SchemaRegistry;
use toolu_orm_core::table::TableSchema;

pub mod shared {
  use toolu_orm_core::column::Text;
  use toolu_orm_macros::table;

  #[table(name = "project_catalog")]
  pub struct Projects {
    #[column(primary_key, not_null)]
    pub id: Text,
  }

  #[table(name = "project_work_items")]
  #[unique_index("project_work_items_id_project_uidx", id, project_id)]
  #[foreign_key(
    columns(parent_work_item_id, project_id),
    references = "project_work_items(id, project_id)"
  )]
  pub struct ProjectWorkItems {
    #[column(primary_key, not_null)]
    pub id: Text,
    #[column(not_null, references = "project_catalog(id)", on_delete = "cascade")]
    pub project_id: Text,
    pub parent_work_item_id: Text,
  }
}

pub mod evidence_v1 {
  use toolu_orm_core::column::Text;
  use toolu_orm_macros::table;

  #[table(name = "project_evidence")]
  pub struct ProjectEvidence {
    #[column(primary_key, not_null)]
    pub id: Text,
    #[column(not_null)]
    pub project_id: Text,
    #[column(not_null)]
    pub work_item_id: Text,
  }
}

pub mod evidence_v2 {
  use toolu_orm_core::column::Text;
  use toolu_orm_macros::table;

  #[table(name = "project_evidence")]
  #[foreign_key(
    name = "project_evidence_work_item_project_fk",
    columns(work_item_id, project_id),
    references = "project_work_items(id, project_id)",
    on_delete = "cascade"
  )]
  pub struct ProjectEvidence {
    #[column(primary_key, not_null)]
    pub id: Text,
    #[column(not_null)]
    pub project_id: Text,
    #[column(not_null)]
    pub work_item_id: Text,
  }
}

/// Evidence without its composite key.
pub fn registry_v1() -> SchemaRegistry {
  SchemaRegistry::from_tables(vec![
    shared::Projects::table_def(),
    shared::ProjectWorkItems::table_def(),
    evidence_v1::ProjectEvidence::table_def(),
  ])
}

/// Evidence with its cascading composite key.
pub fn registry_v2() -> SchemaRegistry {
  SchemaRegistry::from_tables(vec![
    shared::Projects::table_def(),
    shared::ProjectWorkItems::table_def(),
    evidence_v2::ProjectEvidence::table_def(),
  ])
}

/// Two projects, one work item in `p1`, and one evidence row pointing at it:
/// every row valid under either registry.
pub const SEED: [&str; 4] = [
  "INSERT INTO project_catalog (id) VALUES ('p1'), ('p2')",
  "INSERT INTO project_work_items (id, project_id) VALUES ('w1', 'p1')",
  "INSERT INTO project_work_items (id, project_id, parent_work_item_id) VALUES ('w2', 'p1', 'w1')",
  "INSERT INTO project_evidence (id, project_id, work_item_id) VALUES ('e1', 'p1', 'w1')",
];

/// Evidence for `w1` claimed by the wrong project.
pub const DANGLING_EVIDENCE: &str =
  "INSERT INTO project_evidence (id, project_id, work_item_id) VALUES ('e2', 'p2', 'w1')";

/// A child of `w1` claimed by the wrong project.
pub const DANGLING_PARENT: &str =
  "INSERT INTO project_work_items (id, project_id, parent_work_item_id) VALUES ('w3', 'p2', 'w1')";

/// A work item with no parent: the NULL member skips the self-reference.
pub const NULL_PARENT: &str =
  "INSERT INTO project_work_items (id, project_id, parent_work_item_id) VALUES ('w4', 'p2', NULL)";

/// A fresh `migrations/` directory inside a tempdir. Keep the `TempDir` alive
/// for the test's duration.
///
/// # Errors
///
/// Returns the tempdir or path error.
pub fn migrations_dir() -> Result<(tempfile::TempDir, String), Box<dyn std::error::Error>> {
  let dir = tempfile::tempdir()?;
  let path = dir.path().join("migrations");
  std::fs::create_dir_all(&path)?;
  let path = path.to_str().ok_or("non-UTF8 tempdir path")?.to_owned();
  Ok((dir, path))
}
