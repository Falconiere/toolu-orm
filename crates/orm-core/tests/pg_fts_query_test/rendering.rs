//! What `ts_rank` renders, including weights-first and quote doubling.

use toolu_orm_core::column::Text;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::pg_fts;
use toolu_orm_core::query_column::Column;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const SEARCH: Column<Text> = Column::new("docs", "search_vector");

#[test]
fn ts_rank_without_weights_embeds_the_query() -> TestResult {
  let doc = pg_fts::column_for(Dialect::Postgres, &SEARCH)?;
  let score = pg_fts::ts_rank_tsquery_for(Dialect::Postgres, &doc, "english", "runner", None)?;
  assert_eq!(
    score.sql(),
    r#"ts_rank("docs"."search_vector", to_tsquery('english', 'runner'))"#
  );
  assert_eq!(
    score.desc().to_sql(),
    r#"ts_rank("docs"."search_vector", to_tsquery('english', 'runner')) DESC"#
  );
  Ok(())
}

#[test]
fn ts_rank_weights_render_first_as_real_array() -> TestResult {
  let doc = pg_fts::column_for(Dialect::Postgres, &SEARCH)?;
  let score = pg_fts::ts_rank_tsquery_for(
    Dialect::Postgres,
    &doc,
    "english",
    "runner",
    Some(&[0.0, 0.0, 0.0, 1.0]),
  )?;
  assert_eq!(
    score.sql(),
    r#"ts_rank('{0.0,0.0,0.0,1.0}'::real[], "docs"."search_vector", to_tsquery('english', 'runner'))"#
  );
  Ok(())
}

#[test]
fn a_quote_inside_the_rank_query_is_doubled() -> TestResult {
  let doc = pg_fts::column_for(Dialect::Postgres, &SEARCH)?;
  let score = pg_fts::ts_rank_tsquery_for(Dialect::Postgres, &doc, "english", "it's", None)?;
  assert_eq!(
    score.sql(),
    r#"ts_rank("docs"."search_vector", to_tsquery('english', 'it''s'))"#
  );
  Ok(())
}

#[test]
fn plainto_and_websearch_rank_variants_render() -> TestResult {
  let doc = pg_fts::column_for(Dialect::Postgres, &SEARCH)?;
  let plain =
    pg_fts::ts_rank_plainto_tsquery_for(Dialect::Postgres, &doc, "english", "marathon", None)?;
  assert!(plain.sql().contains("plainto_tsquery"));
  let web =
    pg_fts::ts_rank_websearch_to_tsquery_for(Dialect::Postgres, &doc, "english", "runner", None)?;
  assert!(web.sql().contains("websearch_to_tsquery"));
  Ok(())
}

#[test]
fn the_short_rank_forms_agree_with_the_current_dialect() {
  let Ok(doc) = pg_fts::column_for(Dialect::CURRENT, &SEARCH) else {
    // Sqlite CURRENT — construction already refused upstream for short forms
    // that need a document; still check rank short vs explicit.
    let short = pg_fts::column(&SEARCH);
    let explicit = pg_fts::column_for(Dialect::CURRENT, &SEARCH);
    assert_eq!(short.is_ok(), explicit.is_ok());
    return;
  };
  let short = pg_fts::ts_rank_tsquery(&doc, "english", "runner", None);
  let explicit = pg_fts::ts_rank_tsquery_for(Dialect::CURRENT, &doc, "english", "runner", None);
  assert_eq!(short.is_ok(), explicit.is_ok());
}
