//! `#[column(check = "...")]` sets ColumnDef.check and flows through SQL / snapshot.

use toolu_orm_core::column::{ColumnType, Integer, Text};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::diff::Operation;
use toolu_orm_core::schema::SchemaRegistry;
use toolu_orm_core::snapshot::Snapshot;
use toolu_orm_core::sql::generate_sql_for;
use toolu_orm_core::table::TableSchema;
use toolu_orm_macros::table;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[table(name = "ratings")]
pub struct Rating {
  #[column(primary_key)]
  pub id: Text,

  #[column(not_null, default = "3", check = "quality BETWEEN 1 AND 5")]
  pub quality: Integer,

  #[column(
    not_null,
    check = "kind IN ('decision','bug','convention','discovery','pattern','note')"
  )]
  pub kind: Text,
}

#[test]
fn column_check_attr_sets_wrapped_check() -> TestResult {
  let def = Rating::table_def();
  let quality = def.find_column("quality").ok_or("missing quality")?;
  assert_eq!(quality.column_type, ColumnType::Integer);
  assert_eq!(
    quality.check.as_deref(),
    Some("CHECK (quality BETWEEN 1 AND 5)")
  );

  let kind = def.find_column("kind").ok_or("missing kind")?;
  assert_eq!(
    kind.check.as_deref(),
    Some("CHECK (kind IN ('decision','bug','convention','discovery','pattern','note'))")
  );
  Ok(())
}

#[test]
fn column_check_renders_in_create_table_sql() {
  let def = Rating::table_def();
  let sql = generate_sql_for(&[Operation::CreateTable { table: def }], Dialect::Sqlite);
  assert!(
    sql.contains("CHECK (quality BETWEEN 1 AND 5)"),
    "sql: {sql}"
  );
  assert!(
    sql.contains("CHECK (kind IN ('decision','bug','convention','discovery','pattern','note'))"),
    "sql: {sql}"
  );
}

#[test]
fn column_check_flows_through_snapshot_unchanged() -> TestResult {
  let def = Rating::table_def();
  let registry = SchemaRegistry::from_tables(vec![def]);
  let snapshot = Snapshot::from_registry(&registry);
  let ratings = snapshot.tables.get("ratings").ok_or("missing ratings")?;

  assert_eq!(ratings.check_constraints.len(), 2);
  assert_eq!(
    ratings.check_constraints.get("quality").map(String::as_str),
    Some("CHECK (quality BETWEEN 1 AND 5)")
  );
  assert_eq!(
    ratings.check_constraints.get("kind").map(String::as_str),
    Some("CHECK (kind IN ('decision','bug','convention','discovery','pattern','note'))")
  );

  let quality_col = ratings
    .columns
    .get("quality")
    .ok_or("missing quality col")?;
  assert!(
    quality_col.check.is_none(),
    "snapshot columns strip inline check; it lives in check_constraints"
  );
  Ok(())
}
