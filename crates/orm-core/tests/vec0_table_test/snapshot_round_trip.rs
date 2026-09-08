//! The dimension is part of the column's identity, so it has to survive JSON.

use toolu_orm_core::column::{ColumnType, VectorElement};
use toolu_orm_core::schema::SchemaRegistry;
use toolu_orm_core::snapshot::Snapshot;

use crate::{memory_vec, TestResult};

#[test]
fn snapshot_round_trip_keeps_the_dimension_and_the_arguments() -> TestResult {
  let registry = SchemaRegistry::from_tables(vec![memory_vec().build()?]);
  let dir = tempfile::tempdir()?;
  let path = dir.path().join("vec0.snapshot.json");
  let path = path.to_str().ok_or("non-UTF8 path")?;
  Snapshot::from_registry(&registry).write_to_path(path)?;

  assert_eq!(
    serde_json::to_string(&ColumnType::Vector {
      element: VectorElement::Float,
      dim: 1024,
    })?,
    r#"{"Vector":{"element":"float","dim":1024}}"#
  );

  let loaded = Snapshot::read_from_path(path)?;
  let stored = loaded
    .tables
    .get("memory_vec")
    .and_then(|table| table.columns.get("embedding"))
    .ok_or("missing embedding")?;
  assert_eq!(
    stored.column_type,
    ColumnType::Vector {
      element: VectorElement::Float,
      dim: 1024,
    }
  );

  let restored = loaded.to_registry();
  let restored = restored.find_table("memory_vec").ok_or("missing table")?;
  assert_eq!(restored, &memory_vec().build()?);
  Ok(())
}
