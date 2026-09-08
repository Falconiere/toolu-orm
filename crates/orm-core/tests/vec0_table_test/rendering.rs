//! What the builder puts into `TableKind::Virtual` and into `TableDef::columns`.

use toolu_orm_core::column::{ColumnType, VectorElement};
use toolu_orm_core::vec0::{Vec0KeyType, Vec0Table};

use crate::{memory_vec, TestResult};

#[test]
fn builder_renders_one_argument_per_column_in_declaration_order() -> TestResult {
  let table = memory_vec().build()?;
  assert_eq!(table.kind.module(), Some("vec0"));
  assert_eq!(
    table.kind.args(),
    [
      "memory_id text primary key",
      "embedding float[1024] distance_metric=cosine",
      "user_id integer partition key",
      "label text",
      "+contents text",
    ]
  );
  assert!(table.is_virtual());
  assert!(table.indexes.is_empty());
  Ok(())
}

#[test]
fn a_vector_without_a_metric_omits_the_distance_metric() -> TestResult {
  let table = Vec0Table::new("code_vec")
    .primary_key("symbol_id", Vec0KeyType::Integer)
    .vector("embedding", VectorElement::Int8, 768)
    .build()?;
  assert_eq!(
    table.kind.args(),
    ["symbol_id integer primary key", "embedding int8[768]"]
  );
  Ok(())
}

/// The dimension and element type live in `ColumnDef` as well as in the
/// rendered argument, so the diff sees a changed dimension as a changed
/// column.
#[test]
fn the_vector_column_carries_its_element_type_and_dimension() -> TestResult {
  let table = memory_vec().build()?;
  let embedding = table.find_column("embedding").ok_or("missing embedding")?;
  assert_eq!(
    embedding.column_type,
    ColumnType::Vector {
      element: VectorElement::Float,
      dim: 1024,
    }
  );
  assert!(!embedding.primary_key);
  let key = table.find_column("memory_id").ok_or("missing memory_id")?;
  assert!(key.primary_key);
  assert_eq!(key.column_type, ColumnType::Text);
  Ok(())
}

/// `#[vec0_table]` runs the same checks at expansion time, so the unchecked
/// terminal must produce exactly what the checked one does.
#[test]
fn build_prevalidated_matches_build_for_a_valid_table() -> TestResult {
  assert_eq!(memory_vec().build_prevalidated(), memory_vec().build()?);
  Ok(())
}
