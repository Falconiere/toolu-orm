//! The `CREATE VIRTUAL TABLE` statement on each dialect.

use toolu_orm_core::column::{ColumnDef, ColumnType, VectorElement};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::table::{TableDef, TableKind};

use crate::{create_sql, memory_vec, TestResult};

#[test]
fn sqlite_ddl_creates_the_virtual_table() -> TestResult {
  assert_eq!(
    create_sql(&memory_vec().build()?, Dialect::Sqlite),
    "CREATE VIRTUAL TABLE IF NOT EXISTS \"memory_vec\" USING \"vec0\"(memory_id text primary key, \
     embedding float[1024] distance_metric=cosine, user_id integer partition key, label text, \
     +contents text);"
  );
  Ok(())
}

#[test]
fn sqlite_ddl_omits_types_strict_and_constraints() -> TestResult {
  let mut table = memory_vec().build()?;
  table.strict = true;
  if let Some(first) = table.columns.first_mut() {
    first.not_null = true;
    first.default = Some("'x'".to_owned());
  }
  let sql = create_sql(&table, Dialect::Sqlite);
  assert!(!sql.contains("TEXT"), "column types leaked: {sql}");
  assert!(!sql.contains("STRICT"), "STRICT leaked: {sql}");
  assert!(!sql.contains("PRIMARY KEY"), "constraint leaked: {sql}");
  assert!(!sql.contains("DEFAULT"), "default leaked: {sql}");
  Ok(())
}

#[test]
fn postgres_reports_the_skipped_table_instead_of_emitting_ddl() -> TestResult {
  assert_eq!(
    create_sql(&memory_vec().build()?, Dialect::Postgres),
    "-- virtual table \"memory_vec\" USING \"vec0\" is SQLite-only; skipped for postgres"
  );
  Ok(())
}

/// Outside a `vec0` table the value is just its bytes, so an ordinary table
/// with a `Vector` column is legal DDL on both dialects.
#[test]
fn a_vector_column_on_an_ordinary_table_is_a_byte_column() {
  let table = TableDef {
    name: "cached".to_owned(),
    columns: vec![ColumnDef {
      name: "embedding".to_owned(),
      column_type: ColumnType::Vector {
        element: VectorElement::Float,
        dim: 4,
      },
      primary_key: false,
      not_null: false,
      default: None,
      unique: false,
      references: None,
      on_delete: None,
      on_update: None,
      check: None,
      unindexed: false,
      autoincrement: false,
    }],
    indexes: vec![],
    primary_key: vec![],
    strict: false,
    kind: TableKind::Ordinary,
  };
  assert!(create_sql(&table, Dialect::Sqlite).contains("\"embedding\" BLOB"));
  assert!(create_sql(&table, Dialect::Postgres).contains("\"embedding\" BYTEA"));
}
