use toolu_orm_core::column::{Text, Vector};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::diff::Operation;
use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::expr::Expr;
use toolu_orm_core::pg_fts;
use toolu_orm_core::pgvector;
use toolu_orm_core::query_column::Column;
use toolu_orm_core::sql::generate_sql_for;
use toolu_orm_core::vec0;

const BODY: Column<Text> = Column::new("items", "body");
const EMBEDDING: Column<Vector> = Column::new("items", "embedding");

#[test]
fn lance_parameters_and_identifiers_have_duckdb_syntax() {
  assert_eq!(Dialect::Lance.as_str(), "lance");
  assert_eq!(Dialect::Lance.param(2), "?2");
  assert_eq!(Dialect::Lance.now_epoch(), "floor(epoch(now()))::bigint");
  assert_eq!(Dialect::Lance.quote_ident("a\"b"), "\"a\"\"b\"");
  assert_eq!(Dialect::Sqlite.quote_ident("a\"b"), "\"a\"\"b\"");
  assert_eq!(Dialect::Postgres.quote_ident("a\"b"), "\"a\"\"b\"");
}

#[test]
fn engine_specific_query_constructors_refuse_lance() {
  assert!(matches!(
    Expr::table_match_for(Dialect::Lance, "items_fts", "fox"),
    Err(DbCoreError::Fts5UnsupportedDialect {
      dialect: "lance",
      ..
    })
  ));
  assert!(matches!(
    vec0::k_eq_for(Dialect::Lance, 2),
    Err(DbCoreError::Vec0UnsupportedDialect {
      dialect: "lance",
      ..
    })
  ));
  assert!(matches!(
    pg_fts::column_for(Dialect::Lance, &BODY),
    Err(DbCoreError::PgFtsUnsupportedDialect {
      dialect: "lance",
      ..
    })
  ));
  assert!(matches!(
    pgvector::l2_distance_for(Dialect::Lance, &EMBEDDING, &[0.0, 1.0]),
    Err(DbCoreError::PgVectorUnsupportedDialect {
      dialect: "lance",
      ..
    })
  ));
}

#[test]
fn lance_migration_generation_stays_explicitly_unsupported() {
  let sql = generate_sql_for(
    &[Operation::DropTable {
      name: "items".to_owned(),
    }],
    Dialect::Lance,
  );
  assert_eq!(sql, "-- Lance migration SQL is unsupported; see issue #181");
}
