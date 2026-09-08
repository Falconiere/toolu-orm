//! Everything the FTS5 read surface refuses to build: the Postgres dialect,
//! and each argument the engine would misread.

use toolu_orm_core::column::Text;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::expr::Expr;
use toolu_orm_core::fts5::{self, Highlight, Snippet};
use toolu_orm_core::query_column::{Column, Fts5Ops};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const BODY: Column<Text> = Column::new("memory_fts", "body");

fn snippet_spec(tokens: i32, column_index: i32) -> Snippet<'static> {
  Snippet {
    column_index,
    open: "<b>",
    close: "</b>",
    ellipsis: "…",
    tokens,
  }
}

fn highlight_spec(column_index: i32) -> Highlight<'static> {
  Highlight {
    column_index,
    open: "<b>",
    close: "</b>",
  }
}

/// Asserts the error is a Postgres rejection naming `expected_function`, and
/// hands back its rendered message.
fn unsupported_dialect(
  error: DbCoreError,
  expected_function: &str,
) -> Result<String, Box<dyn std::error::Error>> {
  let message = error.to_string();
  let DbCoreError::Fts5UnsupportedDialect { function, dialect } = error else {
    return Err(format!("expected Fts5UnsupportedDialect, got: {error:?}").into());
  };
  assert_eq!(function, expected_function);
  assert_eq!(dialect, "postgres");
  Ok(message)
}

/// Asserts the error is an invalid-argument rejection naming
/// `expected_function`, and hands back its reason.
fn invalid_argument(
  error: DbCoreError,
  expected_function: &str,
) -> Result<String, Box<dyn std::error::Error>> {
  let DbCoreError::Fts5InvalidArgument { function, reason } = error else {
    return Err(format!("expected Fts5InvalidArgument, got: {error:?}").into());
  };
  assert_eq!(function, expected_function);
  Ok(reason)
}

// ── Dialect ──────────────────────────────────────────────────────────────────

#[test]
fn postgres_is_refused_by_every_auxiliary_function() -> TestResult {
  let table = "memory_fts";

  let bm25 = fts5::bm25_for(Dialect::Postgres, table, &[1.0])
    .err()
    .ok_or("bm25 must refuse Dialect::Postgres")?;
  let message = unsupported_dialect(bm25, "bm25")?;
  assert!(
    message.contains("bm25") && message.contains("postgres"),
    "the message must name the function and the dialect: {message}"
  );
  assert!(
    message.contains("to_tsquery"),
    "the message must point at the Postgres alternative: {message}"
  );

  let rank = fts5::rank_for(Dialect::Postgres, table)
    .err()
    .ok_or("rank must refuse Dialect::Postgres")?;
  unsupported_dialect(rank, "rank")?;

  let snippet = fts5::snippet_for(Dialect::Postgres, table, &snippet_spec(32, 1))
    .err()
    .ok_or("snippet must refuse Dialect::Postgres")?;
  unsupported_dialect(snippet, "snippet")?;

  let highlight = fts5::highlight_for(Dialect::Postgres, table, &highlight_spec(1))
    .err()
    .ok_or("highlight must refuse Dialect::Postgres")?;
  unsupported_dialect(highlight, "highlight")?;
  Ok(())
}

#[test]
fn postgres_is_refused_by_both_match_forms() -> TestResult {
  let table_form = Expr::table_match_for(Dialect::Postgres, "memory_fts", "runner")
    .err()
    .ok_or("table MATCH must refuse Dialect::Postgres")?;
  unsupported_dialect(table_form, "MATCH")?;

  let column_form = BODY
    .matches_for(Dialect::Postgres, "runner")
    .err()
    .ok_or("column MATCH must refuse Dialect::Postgres")?;
  unsupported_dialect(column_form, "MATCH")?;
  Ok(())
}

// ── Weights ──────────────────────────────────────────────────────────────────

/// A negative weight is not merely unusual: probed against a real FTS5 build,
/// `bm25(t, -1.0, 1.0)` returns a *positive* score, which silently inverts the
/// "more negative is better" ordering the whole feature rests on.
#[test]
fn a_weight_that_is_not_finite_and_non_negative_is_refused() -> TestResult {
  for (weight, fragment) in [
    (f64::NAN, "is NaN"),
    (f64::INFINITY, "is infinite"),
    (f64::NEG_INFINITY, "is infinite"),
    (-1.0, "is negative"),
  ] {
    let error = fts5::bm25_for(Dialect::Sqlite, "memory_fts", &[0.0, weight])
      .err()
      .ok_or(format!("weight {weight} must be refused"))?;
    let reason = invalid_argument(error, "bm25")?;
    assert!(
      reason.contains(fragment) && reason.contains("weight #1"),
      "reason must name the offending weight and why: {reason}"
    );
  }
  Ok(())
}

// ── Identifiers ──────────────────────────────────────────────────────────────

#[test]
fn a_table_name_that_is_not_a_plain_identifier_is_refused() -> TestResult {
  for table in [
    "",
    "a\"b",
    "a; DROP TABLE t --",
    "memory fts",
    "9lives",
    "mémoire",
  ] {
    let error = fts5::bm25_for(Dialect::Sqlite, table, &[1.0])
      .err()
      .ok_or(format!("table name {table:?} must be refused"))?;
    invalid_argument(error, "bm25")?;

    let match_error = Expr::table_match_for(Dialect::Sqlite, table, "runner")
      .err()
      .ok_or(format!("MATCH on table {table:?} must be refused"))?;
    invalid_argument(match_error, "MATCH")?;
  }
  Ok(())
}

// ── Snippet and highlight arguments ──────────────────────────────────────────

/// FTS5 documents `1..=64` but silently clamps anything outside it, so
/// refusing is the only way the caller learns the number did not take effect.
#[test]
fn a_token_count_outside_the_fts5_range_is_refused() -> TestResult {
  for tokens in [0, -5, 65] {
    let error = fts5::snippet_for(Dialect::Sqlite, "memory_fts", &snippet_spec(tokens, 1))
      .err()
      .ok_or(format!("token count {tokens} must be refused"))?;
    let reason = invalid_argument(error, "snippet")?;
    assert!(
      reason.contains("1..=64"),
      "reason must state the accepted range: {reason}"
    );
  }
  Ok(())
}

#[test]
fn a_column_index_below_minus_one_is_refused_while_minus_one_means_every_column() -> TestResult {
  let error = fts5::highlight_for(Dialect::Sqlite, "memory_fts", &highlight_spec(-2))
    .err()
    .ok_or("column index -2 must be refused")?;
  invalid_argument(error, "highlight")?;

  let all_columns = fts5::highlight_for(Dialect::Sqlite, "memory_fts", &highlight_spec(-1))?;
  assert_eq!(
    all_columns.sql(),
    r#"highlight("memory_fts", -1, '<b>', '</b>')"#
  );
  Ok(())
}

#[test]
fn an_unknown_column_name_is_refused_and_names_the_columns_that_exist() -> TestResult {
  let columns = ["memory_id", "body", "tags"];
  let error = fts5::column_index(&columns, "headline")
    .err()
    .ok_or("an absent column must be refused")?;
  let reason = invalid_argument(error, "column_index")?;
  assert!(
    reason.contains("headline") && reason.contains("body"),
    "reason must name the missing column and the ones that exist: {reason}"
  );
  Ok(())
}
