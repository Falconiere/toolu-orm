//! Everything the Postgres FTS surface refuses: SQLite dialect and bad args.

use toolu_orm_core::column::Text;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::pg_fts;
use toolu_orm_core::query_column::Column;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const SEARCH: Column<Text> = Column::new("docs", "search_vector");
const BODY: Column<Text> = Column::new("docs", "body");

fn unsupported_dialect(
  error: DbCoreError,
  expected_function: &str,
) -> Result<String, Box<dyn std::error::Error>> {
  let message = error.to_string();
  let DbCoreError::PgFtsUnsupportedDialect { function, dialect } = error else {
    return Err(format!("expected PgFtsUnsupportedDialect, got: {error:?}").into());
  };
  assert_eq!(function, expected_function);
  assert_eq!(dialect, "sqlite");
  Ok(message)
}

fn invalid_argument(
  error: DbCoreError,
  expected_function: &str,
) -> Result<String, Box<dyn std::error::Error>> {
  let DbCoreError::PgFtsInvalidArgument { function, reason } = error else {
    return Err(format!("expected PgFtsInvalidArgument, got: {error:?}").into());
  };
  assert_eq!(function, expected_function);
  Ok(reason)
}

#[test]
fn sqlite_is_refused_by_every_constructor() -> TestResult {
  let col = pg_fts::column_for(Dialect::Sqlite, &SEARCH)
    .err()
    .ok_or("column must refuse Sqlite")?;
  let message = unsupported_dialect(col, "tsvector column")?;
  assert!(message.contains("sqlite") && message.contains("MATCH"));

  let tsv = pg_fts::to_tsvector_for(Dialect::Sqlite, "english", &BODY)
    .err()
    .ok_or("to_tsvector must refuse Sqlite")?;
  unsupported_dialect(tsv, "to_tsvector")?;

  // Need a Postgres document to reach the match/rank entry points' dialect check.
  let doc = pg_fts::column_for(Dialect::Postgres, &SEARCH)?;
  let m = doc
    .matches_tsquery_for(Dialect::Sqlite, "english", "runner")
    .err()
    .ok_or("matches_tsquery must refuse Sqlite")?;
  unsupported_dialect(m, "to_tsquery")?;

  let plain = doc
    .matches_plainto_tsquery_for(Dialect::Sqlite, "english", "runner")
    .err()
    .ok_or("plainto must refuse Sqlite")?;
  unsupported_dialect(plain, "plainto_tsquery")?;

  let web = doc
    .matches_websearch_to_tsquery_for(Dialect::Sqlite, "english", "runner")
    .err()
    .ok_or("websearch must refuse Sqlite")?;
  unsupported_dialect(web, "websearch_to_tsquery")?;

  let rank = pg_fts::ts_rank_tsquery_for(Dialect::Sqlite, &doc, "english", "runner", None)
    .err()
    .ok_or("ts_rank must refuse Sqlite")?;
  unsupported_dialect(rank, "ts_rank")?;
  Ok(())
}

#[test]
fn an_invalid_config_is_refused() -> TestResult {
  let empty = pg_fts::to_tsvector_for(Dialect::Postgres, "", &BODY)
    .err()
    .ok_or("empty config")?;
  let reason = invalid_argument(empty, "to_tsvector")?;
  assert!(reason.contains("empty"));

  let spaced = pg_fts::to_tsvector_for(Dialect::Postgres, "en us", &BODY)
    .err()
    .ok_or("spaced config")?;
  invalid_argument(spaced, "to_tsvector")?;

  let doc = pg_fts::column_for(Dialect::Postgres, &SEARCH)?;
  let bad = doc
    .matches_tsquery_for(Dialect::Postgres, "a;b", "runner")
    .err()
    .ok_or("semicolon config")?;
  invalid_argument(bad, "to_tsquery")?;
  Ok(())
}

#[test]
fn a_weight_that_is_not_finite_and_non_negative_is_refused() -> TestResult {
  let doc = pg_fts::column_for(Dialect::Postgres, &SEARCH)?;

  let nan = pg_fts::ts_rank_tsquery_for(
    Dialect::Postgres,
    &doc,
    "english",
    "runner",
    Some(&[f32::NAN, 0.0, 0.0, 1.0]),
  )
  .err()
  .ok_or("NaN weight")?;
  invalid_argument(nan, "ts_rank")?;

  let neg = pg_fts::ts_rank_tsquery_for(
    Dialect::Postgres,
    &doc,
    "english",
    "runner",
    Some(&[-1.0, 0.0, 0.0, 1.0]),
  )
  .err()
  .ok_or("negative weight")?;
  invalid_argument(neg, "ts_rank")?;

  let inf = pg_fts::ts_rank_tsquery_for(
    Dialect::Postgres,
    &doc,
    "english",
    "runner",
    Some(&[f32::INFINITY, 0.0, 0.0, 1.0]),
  )
  .err()
  .ok_or("infinite weight")?;
  invalid_argument(inf, "ts_rank")?;
  Ok(())
}

#[test]
fn fts5_still_refuses_postgres_and_pg_fts_refuses_sqlite() -> TestResult {
  use toolu_orm_core::expr::Expr;
  use toolu_orm_core::fts5;

  let fts5_err = Expr::table_match_for(Dialect::Postgres, "memory_fts", "runner")
    .err()
    .ok_or("fts5 must still refuse Postgres")?;
  assert!(matches!(
    fts5_err,
    DbCoreError::Fts5UnsupportedDialect {
      ref function,
      dialect
    } if function == "MATCH" && dialect == "postgres"
  ));

  let bm25 = fts5::bm25_for(Dialect::Postgres, "memory_fts", &[1.0])
    .err()
    .ok_or("bm25 must still refuse Postgres")?;
  assert!(matches!(
    bm25,
    DbCoreError::Fts5UnsupportedDialect {
      ref function,
      dialect
    } if function == "bm25" && dialect == "postgres"
  ));

  let pg = pg_fts::column_for(Dialect::Sqlite, &SEARCH)
    .err()
    .ok_or("pg_fts must refuse Sqlite")?;
  assert!(matches!(
    pg,
    DbCoreError::PgFtsUnsupportedDialect {
      ref function,
      dialect
    } if function == "tsvector column" && dialect == "sqlite"
  ));
  Ok(())
}
