//! The schemas these suites generate from, and the in-memory database they
//! apply to.

use toolu_orm_connection::{Database, LibsqlConnection};
use toolu_orm_core::column::{ColumnDef, ColumnType, VectorElement};
use toolu_orm_core::fts5::Fts5Table;
use toolu_orm_core::schema::SchemaRegistry;
use toolu_orm_core::table::{TableDef, TableKind};
use toolu_orm_core::vec0::{
  DistanceMetric, Vec0AuxiliaryType, Vec0KeyType, Vec0MetadataType, Vec0Table,
};

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

pub fn migrations_dir() -> Result<(tempfile::TempDir, String), Box<dyn std::error::Error>> {
  let tmp = tempfile::tempdir()?;
  let dir = tmp.path().join("migrations");
  std::fs::create_dir(&dir)?;
  let path = dir.to_str().ok_or("non-UTF8 path")?.to_owned();
  Ok((tmp, path))
}

pub fn memories() -> TableDef {
  let column = |name: &str, primary_key: bool| ColumnDef {
    name: name.to_owned(),
    column_type: ColumnType::Text,
    primary_key,
    not_null: primary_key,
    default: None,
    unique: false,
    references: None,
    on_delete: None,
    on_update: None,
    check: None,
    unindexed: false,
    autoincrement: false,
  };
  TableDef {
    name: "memories".to_owned(),
    columns: vec![column("id", true), column("body", false)],
    indexes: vec![],
    primary_key: vec![],
    strict: false,
    kind: TableKind::Ordinary,
    fts5_sync: None,
  }
}

pub fn memory_vec(dim: u32, metric: DistanceMetric) -> TableDef {
  Vec0Table::new("memory_vec")
    .primary_key("memory_id", Vec0KeyType::Text)
    .vector_metric("embedding", VectorElement::Float, dim, metric)
    .partition_key("user_id", Vec0KeyType::Integer)
    .metadata("label", Vec0MetadataType::Text)
    .auxiliary("contents", Vec0AuxiliaryType::Text)
    .build_prevalidated()
}

pub fn vec0_registry(dim: u32, metric: DistanceMetric) -> SchemaRegistry {
  SchemaRegistry::from_tables(vec![memories(), memory_vec(dim, metric)])
}

pub fn memory_fts() -> TableDef {
  Fts5Table::new("memory_fts")
    .unindexed_column("memory_id", ColumnType::Text)
    .column("body", ColumnType::Text)
    .tokenize("porter unicode61")
    .build()
}

pub fn fts5_registry() -> SchemaRegistry {
  SchemaRegistry::from_tables(vec![memories(), memory_fts()])
}

pub async fn connect() -> Result<LibsqlConnection, Box<dyn std::error::Error>> {
  Ok(Database::init_local(":memory:").await?.connect()?)
}

pub async fn scalar(conn: &LibsqlConnection, sql: &str) -> Result<i64, Box<dyn std::error::Error>> {
  let mut rows = conn.inner_conn().query(sql, ()).await?;
  let row = rows.next().await?.ok_or("scalar query returned no row")?;
  Ok(row.get::<i64>(0)?)
}
