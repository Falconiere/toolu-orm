//! What `bm25`, `rank`, `snippet` and `highlight` render, and how they order.

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::OrderBy;
use toolu_orm_core::fts5::{self, Highlight, Snippet};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn snippet_spec() -> Snippet<'static> {
  Snippet {
    column_index: 1,
    open: "<b>",
    close: "</b>",
    ellipsis: "…",
    tokens: 32,
  }
}

/// Every weight is a float literal. `0.0` must not collapse to `0`: Display
/// would render the integer literal, and only `{:?}` keeps the decimal point.
#[test]
fn bm25_renders_every_weight_as_a_float_literal() -> TestResult {
  let score = fts5::bm25_for(Dialect::Sqlite, "memory_fts", &[0.0, 1.0, 3.0])?;
  assert_eq!(score.sql(), r#"bm25("memory_fts", 0.0, 1.0, 3.0)"#);

  let tiny = fts5::bm25_for(Dialect::Sqlite, "memory_fts", &[1e-10])?;
  assert_eq!(tiny.sql(), r#"bm25("memory_fts", 1e-10)"#);
  Ok(())
}

/// FTS5 reads a weightless call as 1.0 for every column, so an empty slice is
/// a legitimate call rather than a mistake.
#[test]
fn bm25_without_weights_omits_the_argument_list() -> TestResult {
  let score = fts5::bm25_for(Dialect::Sqlite, "memory_fts", &[])?;
  assert_eq!(score.sql(), r#"bm25("memory_fts")"#);
  Ok(())
}

/// `rank` is a hidden column of the table, not a function: `rank(t)` is a
/// syntax error.
#[test]
fn rank_is_the_bare_column_not_a_call() -> TestResult {
  let score = fts5::rank_for(Dialect::Sqlite, "memory_fts")?;
  assert_eq!(score.sql(), "rank");
  Ok(())
}

#[test]
fn snippet_and_highlight_render_their_arguments_in_order() -> TestResult {
  let snip = fts5::snippet_for(Dialect::Sqlite, "memory_fts", &snippet_spec())?;
  assert_eq!(
    snip.sql(),
    r#"snippet("memory_fts", 1, '<b>', '</b>', '…', 32)"#
  );

  let marked = fts5::highlight_for(
    Dialect::Sqlite,
    "memory_fts",
    &Highlight {
      column_index: 2,
      open: "<b>",
      close: "</b>",
    },
  )?;
  assert_eq!(marked.sql(), r#"highlight("memory_fts", 2, '<b>', '</b>')"#);
  Ok(())
}

#[test]
fn a_quote_inside_a_tag_is_doubled() -> TestResult {
  let marked = fts5::highlight_for(
    Dialect::Sqlite,
    "memory_fts",
    &Highlight {
      column_index: 0,
      open: "it's",
      close: "</b>",
    },
  )?;
  assert_eq!(
    marked.sql(),
    r#"highlight("memory_fts", 0, 'it''s', '</b>')"#
  );
  Ok(())
}

/// `bm25()` is negative and more negative is better, so `ASC` is best first.
#[test]
fn a_call_orders_ascending_and_descending() -> TestResult {
  let score = fts5::bm25_for(Dialect::Sqlite, "memory_fts", &[1.0])?;
  assert_eq!(score.asc().to_sql(), r#"bm25("memory_fts", 1.0) ASC"#);
  assert_eq!(score.desc().to_sql(), r#"bm25("memory_fts", 1.0) DESC"#);
  Ok(())
}

#[test]
fn an_alias_orders_by_the_computed_output() {
  assert_eq!(OrderBy::alias_asc("score").to_sql(), r#""score" ASC"#);
  assert_eq!(OrderBy::alias_desc("score").to_sql(), r#""score" DESC"#);
}

#[test]
fn column_index_finds_the_column_by_declaration_order() -> TestResult {
  let columns = ["memory_id", "body", "tags"];
  assert_eq!(fts5::column_index(&columns, "memory_id")?, 0);
  assert_eq!(fts5::column_index(&columns, "tags")?, 2);
  Ok(())
}

/// The short forms follow `Dialect::CURRENT` — Sqlite on the default lane,
/// Postgres on the postgres lane — so agreement is what both lanes can assert.
#[test]
fn the_short_forms_agree_with_the_current_dialect() {
  let table = "memory_fts";
  assert_eq!(
    fts5::bm25(table, &[1.0]).is_ok(),
    fts5::bm25_for(Dialect::CURRENT, table, &[1.0]).is_ok()
  );
  assert_eq!(
    fts5::rank(table).is_ok(),
    fts5::rank_for(Dialect::CURRENT, table).is_ok()
  );
  assert_eq!(
    fts5::snippet(table, &snippet_spec()).is_ok(),
    fts5::snippet_for(Dialect::CURRENT, table, &snippet_spec()).is_ok()
  );
  let spec = Highlight {
    column_index: 1,
    open: "<b>",
    close: "</b>",
  };
  assert_eq!(
    fts5::highlight(table, &spec).is_ok(),
    fts5::highlight_for(Dialect::CURRENT, table, &spec).is_ok()
  );
}
