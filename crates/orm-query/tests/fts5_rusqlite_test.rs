//! The FTS5 read surface against a real in-memory rusqlite index: what the
//! `bm25` weights actually do to the returned order, and the markup `snippet`
//! and `highlight` produce.

#[path = "fixtures/fts5_schema.rs"]
pub mod schema;

use toolu_orm_core::column::Text;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::{Expr, OrderBy};
use toolu_orm_core::fts5::{self, Highlight, Snippet};
use toolu_orm_core::query_column::{Column, CommonOps, Fts5Ops};
use toolu_orm_macros::FromRow;
use toolu_orm_query::select::SelectBuilder;

use schema::{CREATE_MEMORIES_SQL, FTS_COLUMNS, ROWS};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const TABLE: &str = "memory_fts";
const FTS_MEMORY_ID: Column<Text> = Column::new("memory_fts", "memory_id");
const FTS_BODY: Column<Text> = Column::new("memory_fts", "body");
const MEMORY_ID: Column<Text> = Column::new("memories", "id");
const MEMORY_DELETED_AT: Column<Text> = Column::new("memories", "deleted_at");

#[derive(FromRow, Debug, Clone, PartialEq)]
struct Hit {
  memory_id: String,
}

#[derive(FromRow, Debug, Clone, PartialEq)]
struct ScoredHit {
  memory_id: String,
  score: f64,
}

#[derive(FromRow, Debug, Clone, PartialEq)]
struct MarkedHit {
  memory_id: String,
  marked: String,
}

fn seeded_conn() -> Result<rusqlite::Connection, Box<dyn std::error::Error>> {
  let conn = rusqlite::Connection::open_in_memory()?;
  conn.execute_batch(&schema::create_fts_sql())?;
  conn.execute_batch(CREATE_MEMORIES_SQL)?;
  for (id, body, tags, deleted) in ROWS {
    conn.execute(
      "INSERT INTO memory_fts (memory_id, body, tags) VALUES (?1, ?2, ?3)",
      (id, body, tags),
    )?;
    conn.execute(
      "INSERT INTO memories (id, deleted_at) VALUES (?1, ?2)",
      (id, if deleted { Some(1_i64) } else { None }),
    )?;
  }
  Ok(conn)
}

fn ranked_by(
  conn: &rusqlite::Connection,
  weights: &[f64],
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
  let score = fts5::bm25_for(Dialect::Sqlite, TABLE, weights)?;
  let hits: Vec<ScoredHit> = SelectBuilder::new(TABLE)
    .columns_raw(&["memory_id"])
    .column_expr(score.sql(), "score")
    .filter(Expr::table_match_for(Dialect::Sqlite, TABLE, "runner")?)
    .order_by(OrderBy::alias_asc("score"))
    .fetch_all(conn)?;
  Ok(hits.into_iter().map(|h| h.memory_id).collect())
}

/// The reason typed weights belong in the ORM: changing them changes the
/// answer. `m2` carries the term in `body`, `m3` in `tags`, so weighting one
/// column above the other swaps which row comes back first.
#[test]
fn column_weights_change_the_returned_order() -> TestResult {
  let conn = seeded_conn()?;

  let body_heavy = ranked_by(&conn, &[0.0, 3.0, 1.0])?;
  assert_eq!(body_heavy, vec!["m2", "m3"]);

  let tags_heavy = ranked_by(&conn, &[0.0, 1.0, 3.0])?;
  assert_eq!(tags_heavy, vec!["m3", "m2"]);
  Ok(())
}

/// A `0.0` weight must reach SQL as a float literal; if it collapsed to the
/// integer `0` the call would still parse, so the proof is behavioural — the
/// zero-weighted column stops contributing and the other row wins.
#[test]
fn a_zero_weight_excludes_its_column_from_the_score() -> TestResult {
  let conn = seeded_conn()?;
  assert_eq!(ranked_by(&conn, &[0.0, 0.0, 1.0])?, vec!["m3", "m2"]);
  assert_eq!(ranked_by(&conn, &[0.0, 1.0, 0.0])?, vec!["m2", "m3"]);
  Ok(())
}

#[test]
fn the_issue_query_runs_and_excludes_the_soft_deleted_row() -> TestResult {
  let conn = seeded_conn()?;
  let score = fts5::bm25_for(Dialect::Sqlite, TABLE, &[0.0, 3.0, 1.0])?;

  let hits: Vec<ScoredHit> = SelectBuilder::new(TABLE)
    .columns_raw(&["memory_id"])
    .column_expr(score.sql(), "score")
    .join("memories", MEMORY_ID.equals(&FTS_MEMORY_ID))
    .filter(Expr::table_match_for(Dialect::Sqlite, TABLE, "runner")?)
    .filter(MEMORY_DELETED_AT.is_null())
    .order_by(OrderBy::alias_asc("score"))
    .limit(10)
    .fetch_all(&conn)?;

  let ids: Vec<&str> = hits.iter().map(|h| h.memory_id.as_str()).collect();
  assert_eq!(ids, vec!["m2", "m3"]);
  assert!(hits.iter().all(|h| h.score < 0.0), "bm25 must be negative");
  Ok(())
}

#[test]
fn snippet_marks_up_the_matched_term() -> TestResult {
  let conn = seeded_conn()?;
  let body = fts5::column_index(&FTS_COLUMNS, "body")?;
  let snip = fts5::snippet_for(
    Dialect::Sqlite,
    TABLE,
    &Snippet {
      column_index: body,
      open: "<b>",
      close: "</b>",
      ellipsis: "…",
      tokens: 8,
    },
  )?;

  let hits: Vec<MarkedHit> = SelectBuilder::new(TABLE)
    .columns_raw(&["memory_id"])
    .column_expr(snip.sql(), "marked")
    .filter(FTS_BODY.matches_for(Dialect::Sqlite, "runner")?)
    .fetch_all(&conn)?;

  let hit = hits.first().ok_or("snippet query returned no rows")?;
  assert_eq!(hit.memory_id, "m2");
  assert_eq!(hit.marked, "a marathon <b>runner</b> trains daily");
  Ok(())
}

#[test]
fn highlight_marks_up_the_whole_column() -> TestResult {
  let conn = seeded_conn()?;
  let tags = fts5::column_index(&FTS_COLUMNS, "tags")?;
  let marked = fts5::highlight_for(
    Dialect::Sqlite,
    TABLE,
    &Highlight {
      column_index: tags,
      open: "<b>",
      close: "</b>",
    },
  )?;

  let hits: Vec<MarkedHit> = SelectBuilder::new(TABLE)
    .columns_raw(&["memory_id"])
    .column_expr(marked.sql(), "marked")
    .filter(Expr::table_match_for(Dialect::Sqlite, TABLE, "runner")?)
    .order_by(fts5::rank_for(Dialect::Sqlite, TABLE)?.asc())
    .fetch_all(&conn)?;

  let tagged = hits
    .iter()
    .find(|h| h.memory_id == "m3")
    .ok_or("m3 must match 'runner' through its tags")?;
  assert_eq!(tagged.marked, "<b>runner</b> gear");
  Ok(())
}

/// The read side agrees with #18's write side: `memory_id` is `UNINDEXED`.
#[test]
fn the_unindexed_column_is_not_searchable() -> TestResult {
  let conn = seeded_conn()?;
  let hits: Vec<Hit> = SelectBuilder::new(TABLE)
    .columns_raw(&["memory_id"])
    .filter(Expr::table_match_for(Dialect::Sqlite, TABLE, "m1")?)
    .fetch_all(&conn)?;
  assert!(hits.is_empty());
  Ok(())
}
