//! Live sqlite-vec: register the extension, adopt the connection with
//! `from_connection`, apply ORM `vec0` DDL, insert embeddings, and run
//! `SelectBuilder::knn` against the real module.
//!
//! Compiles only with `--features rusqlite,sqlite-vec` (CI rusqlite-query
//! lane). Without the extension feature this binary is not built.

use toolu_orm_connection::rusqlite_impl::RusqliteConnection;
use toolu_orm_connection::DbConnectionBlocking;
use toolu_orm_core::column::{Vector, VectorElement};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::diff::Operation;
use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::query_column::Column;
use toolu_orm_core::row::FromRow;
use toolu_orm_core::sql::generate_sql_for;
use toolu_orm_core::value::Value;
use toolu_orm_core::vec0::{Vec0KeyType, Vec0Table};
use toolu_orm_query::select::SelectBuilder;
use toolu_orm_sqlite_vec_register as sqlite_vec_register;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const EMBEDDING: Column<Vector> = Column::new("memory_vec", "embedding");

struct Hit {
  memory_id: String,
  distance: f64,
}

impl FromRow for Hit {
  const REQUIRED_COLUMNS: &'static [&'static str] = &[];

  fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, DbCoreError> {
    Ok(Self {
      memory_id: row
        .get(0)
        .map_err(|e| DbCoreError::RowMapping(e.to_string()))?,
      distance: row
        .get(1)
        .map_err(|e| DbCoreError::RowMapping(e.to_string()))?,
    })
  }
}

struct NameRow {
  name: String,
}

impl FromRow for NameRow {
  const REQUIRED_COLUMNS: &'static [&'static str] = &[];

  fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, DbCoreError> {
    Ok(Self {
      name: row
        .get(0)
        .map_err(|e| DbCoreError::RowMapping(e.to_string()))?,
    })
  }
}

struct IdRow;

impl FromRow for IdRow {
  const REQUIRED_COLUMNS: &'static [&'static str] = &[];

  fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, DbCoreError> {
    let _: String = row
      .get(0)
      .map_err(|e| DbCoreError::RowMapping(e.to_string()))?;
    Ok(Self)
  }
}

fn memory_vec_ddl() -> String {
  let table = Vec0Table::new("memory_vec")
    .primary_key("memory_id", Vec0KeyType::Text)
    .vector("embedding", VectorElement::Float, 4)
    .build_prevalidated();
  generate_sql_for(&[Operation::CreateTable { table }], Dialect::Sqlite)
}

fn seeded_conn() -> Result<RusqliteConnection, Box<dyn std::error::Error>> {
  sqlite_vec_register::register();
  let raw = rusqlite::Connection::open_in_memory()?;
  let conn = RusqliteConnection::from_connection(raw);

  DbConnectionBlocking::execute_batch(&conn, &memory_vec_ddl())?;

  // Distinct directions so L2 (vec0 default) ranks them; equal-direction
  // vectors are cosine-ties and would not prove nearest-neighbour order.
  let rows = [
    ("near", [0.1f32, 0.0, 0.0, 0.0]),
    ("mid", [0.5, 0.0, 0.0, 0.0]),
    ("far", [0.9, 0.0, 0.0, 0.0]),
  ];
  for (id, embedding) in rows {
    DbConnectionBlocking::execute_sql(
      &conn,
      r#"INSERT INTO "memory_vec" ("memory_id", "embedding") VALUES (?1, ?2)"#,
      vec![Value::Text(id.to_owned()), Value::vector(&embedding)],
    )?;
  }
  Ok(conn)
}

#[test]
fn from_connection_keeps_sqlite_vec_so_vec0_ddl_applies() -> TestResult {
  let conn = seeded_conn()?;
  let names: Vec<NameRow> = DbConnectionBlocking::query_map(
    &conn,
    r#"SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'memory_vec'"#,
    vec![],
  )?;
  assert_eq!(
    names.first().map(|row| row.name.as_str()),
    Some("memory_vec")
  );
  Ok(())
}

#[test]
fn knn_returns_the_nearest_seeded_row_first() -> TestResult {
  let conn = seeded_conn()?;
  let query = Value::vector(&[0.12f32, 0.0, 0.0, 0.0]);
  let distance = toolu_orm_core::vec0::distance_for(Dialect::Sqlite)?;

  let (sql, params) = SelectBuilder::new("memory_vec")
    .columns_raw(&["memory_id"])
    .column_expr(distance.sql(), "distance")
    .knn_for(Dialect::Sqlite, &EMBEDDING, query, 2)?
    .order_by(distance)
    .to_sql_for(Dialect::Sqlite);

  let hits: Vec<Hit> = DbConnectionBlocking::query_map(&conn, &sql, params)?;
  let ids: Vec<&str> = hits.iter().map(|hit| hit.memory_id.as_str()).collect();
  assert_eq!(ids, vec!["near", "mid"]);

  let near = hits.first().ok_or("missing near")?;
  let mid = hits.get(1).ok_or("missing mid")?;
  // vec0 L2 distance is Euclidean: |0.12-0.1| = 0.02, |0.12-0.5| = 0.38
  assert!(
    (near.distance - 0.02).abs() < 1e-5,
    "near distance: {}",
    near.distance
  );
  assert!(
    (mid.distance - 0.38).abs() < 1e-5,
    "mid distance: {}",
    mid.distance
  );
  assert!(near.distance < mid.distance);
  Ok(())
}

#[test]
fn knn_on_an_empty_vec0_table_returns_no_rows() -> TestResult {
  sqlite_vec_register::register();
  let conn = RusqliteConnection::from_connection(rusqlite::Connection::open_in_memory()?);
  DbConnectionBlocking::execute_batch(&conn, &memory_vec_ddl())?;

  let (sql, params) = SelectBuilder::new("memory_vec")
    .columns_raw(&["memory_id"])
    .knn_for(
      Dialect::Sqlite,
      &EMBEDDING,
      Value::vector(&[0.0f32, 0.0, 0.0, 0.0]),
      3,
    )?
    .to_sql_for(Dialect::Sqlite);

  let hits: Vec<IdRow> = DbConnectionBlocking::query_map(&conn, &sql, params)?;
  assert!(hits.is_empty());
  Ok(())
}
