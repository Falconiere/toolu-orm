//! `Value::vector` through a real rusqlite connection: the little-endian f32
//! bytes land in a BLOB column rendered from `ColumnType::Vector` on an
//! ordinary table (the form Postgres and non-vec0 SQLite use for the same
//! type), and round-trip as the original floats.
#![cfg(all(
  feature = "rusqlite",
  not(feature = "libsql"),
  not(feature = "postgres")
))]

use toolu_orm_connection::DbConnection;
use toolu_orm_connection::rusqlite_impl::RusqliteConnection;
use toolu_orm_core::column::{ColumnDef, ColumnType, VectorElement};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::diff::Operation;
use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::row::FromRow;
use toolu_orm_core::sql::generate_sql_for;
use toolu_orm_core::table::{TableDef, TableKind};
use toolu_orm_core::value::Value;

struct BlobRow {
  value: Vec<u8>,
}

impl FromRow for BlobRow {
  const REQUIRED_COLUMNS: &'static [&'static str] = &[];

  fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, DbCoreError> {
    let value: Vec<u8> = row
      .get(0)
      .map_err(|e| DbCoreError::RowMapping(e.to_string()))?;
    Ok(Self { value })
  }
}

fn embeddings_ddl() -> String {
  let table = TableDef {
    name: "embeddings".to_owned(),
    columns: vec![
      ColumnDef {
        name: "id".to_owned(),
        column_type: ColumnType::Integer,
        primary_key: true,
        not_null: true,
        default: None,
        unique: false,
        references: None,
        on_delete: None,
        on_update: None,
        check: None,
        unindexed: false,
        autoincrement: false,
      },
      ColumnDef {
        name: "vec".to_owned(),
        column_type: ColumnType::Vector {
          element: VectorElement::Float,
          dim: 2,
        },
        primary_key: false,
        not_null: true,
        default: None,
        unique: false,
        references: None,
        on_delete: None,
        on_update: None,
        check: None,
        unindexed: false,
        autoincrement: false,
      },
    ],
    indexes: vec![],
    primary_key: vec![],
    strict: false,
    kind: TableKind::Ordinary,
  };
  generate_sql_for(&[Operation::CreateTable { table }], Dialect::Sqlite)
}

#[tokio::test]
async fn vector_bytes_round_trip_through_a_real_blob_column()
-> Result<(), Box<dyn std::error::Error>> {
  let conn = RusqliteConnection::open_in_memory().await?;
  conn.execute_batch(&embeddings_ddl()).await?;

  let blob = Value::vector(&[1.0f32, 2.0]);
  conn
    .execute_sql(
      "INSERT INTO embeddings (id, vec) VALUES (?1, ?2)",
      vec![Value::Integer(1), blob],
    )
    .await?;

  let rows: Vec<BlobRow> = conn
    .query_map("SELECT vec FROM embeddings WHERE id = 1", vec![])
    .await?;
  let bytes = &rows.first().ok_or("no row")?.value;
  assert_eq!(bytes.len(), 8);
  let mut chunks = bytes.chunks_exact(4);
  let a = f32::from_le_bytes(chunks.next().ok_or("short")?.try_into()?);
  let b = f32::from_le_bytes(chunks.next().ok_or("short")?.try_into()?);
  assert_eq!((a, b), (1.0, 2.0));
  Ok(())
}

#[test]
fn a_mismatched_length_is_refused_before_the_driver() {
  let err = Value::vector_with_dim(&[1.0f32, 2.0], 3).expect_err("length mismatch");
  assert!(
    matches!(
      &err,
      DbCoreError::VectorDimension {
        expected: 3,
        actual: 2
      }
    ),
    "expected VectorDimension, got {err}"
  );
}
