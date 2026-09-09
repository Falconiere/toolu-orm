//! What `ts_rank` renders, including weights-first and dollar-quoting.

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
    r#"ts_rank("docs"."search_vector", to_tsquery('english', $q$runner$q$))"#
  );
  assert_eq!(
    score.desc().to_sql(),
    r#"ts_rank("docs"."search_vector", to_tsquery('english', $q$runner$q$)) DESC"#
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
    r#"ts_rank('{0.0,0.0,0.0,1.0}'::real[], "docs"."search_vector", to_tsquery('english', $q$runner$q$))"#
  );
  Ok(())
}

#[test]
fn a_quote_inside_the_rank_query_is_kept_verbatim() -> TestResult {
  let doc = pg_fts::column_for(Dialect::Postgres, &SEARCH)?;
  let score = pg_fts::ts_rank_tsquery_for(Dialect::Postgres, &doc, "english", "it's", None)?;
  assert_eq!(
    score.sql(),
    r#"ts_rank("docs"."search_vector", to_tsquery('english', $q$it's$q$))"#
  );
  Ok(())
}

#[test]
fn a_backslash_inside_the_rank_query_is_kept_verbatim() -> TestResult {
  let doc = pg_fts::column_for(Dialect::Postgres, &SEARCH)?;
  let score = pg_fts::ts_rank_tsquery_for(Dialect::Postgres, &doc, "english", r"a\b", None)?;
  assert_eq!(
    score.sql(),
    r#"ts_rank("docs"."search_vector", to_tsquery('english', $q$a\b$q$))"#
  );
  Ok(())
}

#[test]
fn a_colliding_dollar_tag_picks_the_next_tag() -> TestResult {
  let doc = pg_fts::column_for(Dialect::Postgres, &SEARCH)?;
  let score = pg_fts::ts_rank_tsquery_for(Dialect::Postgres, &doc, "english", "$q$boom", None)?;
  assert_eq!(
    score.sql(),
    r#"ts_rank("docs"."search_vector", to_tsquery('english', $q1$$q$boom$q1$))"#
  );
  Ok(())
}

#[test]
fn plainto_and_websearch_rank_variants_render() -> TestResult {
  let doc = pg_fts::column_for(Dialect::Postgres, &SEARCH)?;
  let plain =
    pg_fts::ts_rank_plainto_tsquery_for(Dialect::Postgres, &doc, "english", "marathon", None)?;
  assert_eq!(
    plain.sql(),
    r#"ts_rank("docs"."search_vector", plainto_tsquery('english', $q$marathon$q$))"#
  );
  let web =
    pg_fts::ts_rank_websearch_to_tsquery_for(Dialect::Postgres, &doc, "english", "runner", None)?;
  assert_eq!(
    web.sql(),
    r#"ts_rank("docs"."search_vector", websearch_to_tsquery('english', $q$runner$q$))"#
  );
  Ok(())
}

#[test]
fn the_short_rank_forms_agree_with_the_current_dialect() -> TestResult {
  let Ok(doc) = pg_fts::column_for(Dialect::CURRENT, &SEARCH) else {
    let short = pg_fts::column(&SEARCH);
    let explicit = pg_fts::column_for(Dialect::CURRENT, &SEARCH);
    assert_eq!(short.is_ok(), explicit.is_ok());
    return Ok(());
  };
  let short = pg_fts::ts_rank_tsquery(&doc, "english", "runner", None)?;
  let explicit = pg_fts::ts_rank_tsquery_for(Dialect::CURRENT, &doc, "english", "runner", None)?;
  assert_eq!(short.sql(), explicit.sql());
  Ok(())
}
