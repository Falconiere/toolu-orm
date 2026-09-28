//! Table-level composite foreign keys survive a snapshot written to disk and
//! read back, and a snapshot from before they existed still loads.

use toolu_orm_core::diff::diff;
use toolu_orm_core::schema::SchemaRegistry;
use toolu_orm_core::snapshot::Snapshot;
use toolu_orm_core::table::TableDef;

use crate::composite_fk_schema::{evidence_fk, parent_fk, registry};
use crate::registry_samples::TestResult;

fn find<'a>(registry: &'a SchemaRegistry, name: &str) -> Result<&'a TableDef, String> {
  registry
    .tables()
    .iter()
    .find(|t| t.name == name)
    .ok_or_else(|| format!("no table {name}"))
}

#[test]
fn composite_fks_round_trip_through_disk_with_no_diff() -> TestResult {
  let registry = registry();
  let dir = tempfile::tempdir()?;
  let path = dir.path().join("snapshot.json");
  let path = path.to_str().ok_or("non-utf8 temp path")?;
  Snapshot::from_registry(&registry).write_to_path(path)?;

  let read_back = Snapshot::read_from_path(path)?;
  assert_eq!(diff(&read_back, &registry)?, vec![]);

  let restored = read_back.to_registry();
  assert_eq!(
    find(&restored, "project_work_items")?.foreign_keys,
    vec![parent_fk()]
  );
  let evidence = find(&restored, "project_evidence")?;
  assert_eq!(evidence.foreign_keys, vec![evidence_fk()]);
  for column in &evidence.columns {
    assert_eq!(column.references, None, "column {}", column.name);
  }
  Ok(())
}

#[test]
fn column_level_fk_stays_on_its_column_after_to_registry() -> TestResult {
  let restored = Snapshot::from_registry(&registry()).to_registry();
  let work_items = find(&restored, "project_work_items")?;
  let project_id = work_items
    .find_column("project_id")
    .ok_or("no project_id column")?;
  assert_eq!(project_id.references.as_deref(), Some("projects(id)"));
  assert_eq!(work_items.foreign_keys, vec![parent_fk()]);
  Ok(())
}

/// The 0.10.1 shape: a column-level FK in `foreign_keys` and a table
/// definition JSON with no `foreign_keys` key at all.
#[test]
fn pre_composite_snapshot_loads_and_diffs_clean() -> TestResult {
  let json = r#"{
    "version": 1, "dialect": "sqlite", "id": "a", "prev_id": "b",
    "tables": { "posts": {
      "column_order": ["id", "author_id"],
      "columns": {
        "id": {"name": "id", "column_type": "Text", "primary_key": true,
               "not_null": true, "unique": false},
        "author_id": {"name": "author_id", "column_type": "Text", "primary_key": false,
                      "not_null": true, "unique": false}
      },
      "indexes": {},
      "foreign_keys": { "fk_posts_author_id": {
        "name": "fk_posts_author_id", "columns": ["author_id"],
        "references_table": "users", "references_columns": ["id"] } },
      "check_constraints": {},
      "strict": false
    } }
  }"#;
  let snapshot: Snapshot = serde_json::from_str(json)?;
  let registry = snapshot.to_registry();
  let posts = find(&registry, "posts")?;
  assert!(posts.foreign_keys.is_empty());
  let author = posts.find_column("author_id").ok_or("no author_id")?;
  assert_eq!(author.references.as_deref(), Some("users(id)"));
  assert_eq!(diff(&snapshot, &registry)?, vec![]);

  let table_json = serde_json::to_value(posts)?;
  assert!(table_json.get("foreign_keys").is_none(), "{table_json}");
  let reparsed: TableDef = serde_json::from_value(table_json)?;
  assert_eq!(&reparsed, posts);
  Ok(())
}
