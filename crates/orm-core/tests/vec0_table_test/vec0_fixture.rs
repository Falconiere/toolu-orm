//! The table every case in this binary is built from, and the DDL helper.

use toolu_orm_core::column::VectorElement;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::diff::Operation;
use toolu_orm_core::sql::generate_sql_for;
use toolu_orm_core::table::TableDef;
use toolu_orm_core::vec0::{
  DistanceMetric, Vec0AuxiliaryType, Vec0KeyType, Vec0MetadataType, Vec0Table,
};

pub(crate) type TestResult = Result<(), Box<dyn std::error::Error>>;

/// The retrieval table from issue #19: a keyed vector table with one of every
/// non-vector column kind `vec0` supports.
pub(crate) fn memory_vec() -> Vec0Table {
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
}

pub(crate) fn create_sql(table: &TableDef, dialect: Dialect) -> String {
  generate_sql_for(
    &[Operation::CreateTable {
      table: table.clone(),
    }],
    dialect,
  )
}
