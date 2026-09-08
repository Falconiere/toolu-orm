//! `#[vec0_table]` expansion: the virtual `TableDef`, every column role, the
//! typed column module, and the builder factories — asserted equal to the
//! hand-built [`Vec0Table`] so the macro cannot drift from the renderer.

use toolu_orm_core::column::{Integer, Text, Vector, VectorElement};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::table::TableSchema;
use toolu_orm_core::value::Value;
use toolu_orm_core::vec0::{
  DistanceMetric, Vec0AuxiliaryType, Vec0KeyType, Vec0MetadataType, Vec0Table,
};
use toolu_orm_macros::vec0_table;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[vec0_table(name = "memory_vec")]
pub struct MemoryVec {
  #[column(primary_key)]
  pub memory_id: Text,
  #[column(dim = 1024, distance_metric = "cosine")]
  pub embedding: Vector,
  #[column(partition_key)]
  pub user_id: Integer,
  pub label: Text,
  #[column(auxiliary)]
  pub contents: Text,
}

/// Two vector columns is a legal `vec0` shape; the macro must not invent a
/// single-vector restriction.
#[vec0_table(name = "dual_vec")]
pub struct DualVec {
  #[column(primary_key)]
  pub id: Text,
  #[column(dim = 8)]
  pub a: Vector,
  #[column(dim = 16, element = "int8")]
  pub b: Vector,
}

fn hand_built() -> Result<toolu_orm_core::table::TableDef, Box<dyn std::error::Error>> {
  Ok(
    Vec0Table::new("memory_vec")
      .primary_key("memory_id", Vec0KeyType::Text)
      .vector_metric(
        "embedding",
        VectorElement::Float,
        1024,
        DistanceMetric::Cosine,
      )
      .partition_key("user_id", Vec0KeyType::Integer)
      .metadata("label", Vec0MetadataType::Text)
      .auxiliary("contents", Vec0AuxiliaryType::Text)
      .build()?,
  )
}

#[test]
fn table_def_matches_the_hand_built_vec0_table() -> TestResult {
  assert_eq!(MemoryVec::table_def(), hand_built()?);
  Ok(())
}

#[test]
fn table_def_is_a_virtual_vec0_table() {
  let def = MemoryVec::table_def();
  assert_eq!(def.name, "memory_vec");
  assert!(def.is_virtual());
  assert_eq!(def.kind.module(), Some("vec0"));
  assert_eq!(
    def.kind.args(),
    [
      "memory_id text primary key",
      "embedding float[1024] distance_metric=cosine",
      "user_id integer partition key",
      "label text",
      "+contents text",
    ]
  );
  assert!(def.indexes.is_empty());
  assert!(!def.strict);
}

#[test]
fn multiple_vector_columns_are_allowed() {
  let def = DualVec::table_def();
  assert_eq!(def.kind.module(), Some("vec0"));
  assert_eq!(
    def.kind.args(),
    ["id text primary key", "a float[8]", "b int8[16]",]
  );
}

#[test]
fn the_vector_column_carries_its_element_type_and_dimension() -> TestResult {
  let def = MemoryVec::table_def();
  let embedding = def.find_column("embedding").ok_or("missing embedding")?;
  assert_eq!(
    embedding.column_type,
    toolu_orm_core::column::ColumnType::Vector {
      element: VectorElement::Float,
      dim: 1024,
    }
  );
  Ok(())
}

#[test]
fn the_column_module_is_generated() {
  assert_eq!(memory_vec::TABLE, "memory_vec");
  assert_eq!(
    memory_vec::ALL_COLUMNS,
    ["memory_id", "embedding", "user_id", "label", "contents"]
  );
  assert_eq!(memory_vec::embedding.name, "embedding");
  assert_eq!(
    memory_vec::embedding.qualified(),
    "\"memory_vec\".\"embedding\""
  );
}

#[test]
fn the_builder_factories_are_generated() {
  assert_eq!(
    MemoryVec::select()
      .columns_raw(memory_vec::ALL_COLUMNS)
      .to_sql_for(Dialect::Sqlite)
      .0,
    "SELECT \"memory_id\", \"embedding\", \"user_id\", \"label\", \"contents\" FROM \"memory_vec\""
  );
  assert_eq!(
    MemoryVec::insert()
      .set(&memory_vec::label, Value::Text("hello".to_owned()))
      .to_sql_for(Dialect::Sqlite)
      .0,
    "INSERT INTO \"memory_vec\" (\"label\") VALUES (?1)"
  );
  assert_eq!(
    MemoryVec::update()
      .set(&memory_vec::label, Value::Text("hello".to_owned()))
      .to_sql_for(Dialect::Sqlite)
      .0,
    "UPDATE \"memory_vec\" SET \"label\" = ?1"
  );
  assert_eq!(
    MemoryVec::delete().to_sql_for(Dialect::Sqlite).0,
    "DELETE FROM \"memory_vec\""
  );
}
