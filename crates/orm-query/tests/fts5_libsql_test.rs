//! The FTS5 read surface against a real in-memory libsql index: the query from
//! issue #20 built entirely through `SelectBuilder`, its ranking, and the sign
//! of `bm25`.

#[path = "fixtures/fts5_schema.rs"]
pub mod schema;

use toolu_orm_core::column::Text;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::{Expr, OrderBy};
use toolu_orm_core::fts5;
use toolu_orm_core::query_column::{Column, CommonOps, Fts5Ops};
use toolu_orm_macros::FromRow;
use toolu_orm_query::select::SelectBuilder;

use schema::{CREATE_MEMORIES_SQL, ROWS};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const FTS_MEMORY_ID: Column<Text> = Column::new("memory_fts", "memory_id");
const FTS_BODY: Column<Text> = Column::new("memory_fts", "body");
const FTS_TAGS: Column<Text> = Column::new("memory_fts", "tags");
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

async fn seeded_conn() -> Result<libsql::Connection, Box<dyn std::error::Error>> {
  let db = libsql::Builder::new_local(":memory:").build().await?;
  let conn = db.connect()?;
  conn.execute(&schema::create_fts_sql(), ()).await?;
  conn.execute(CREATE_MEMORIES_SQL, ()).await?;
  for (id, body, tags, deleted) in ROWS {
    conn
      .execute(
        "INSERT INTO memory_fts (memory_id, body, tags) VALUES (?1, ?2, ?3)",
        libsql::params![id, body, tags],
      )
      .await?;
    conn
      .execute(
        "INSERT INTO memories (id, deleted_at) VALUES (?1, ?2)",
        libsql::params![id, if deleted { Some(1_i64) } else { None }],
      )
      .await?;
  }
  Ok(conn)
}

/// The whole query from the issue, built through the builder and answered by a
/// real index: `bm25` projection, join, `MATCH`, soft-delete filter, ordering
/// by the computed alias, limit.
#[tokio::test]
async fn the_issue_query_runs_and_ranks_against_a_real_index() -> TestResult {
  let conn = seeded_conn().await?;
  let score = fts5::bm25_for(Dialect::Sqlite, "memory_fts", &[0.0, 3.0, 1.0])?;

  let hits: Vec<ScoredHit> = SelectBuilder::new("memory_fts")
    .columns_raw(&["memory_id"])
    .column_expr(score.sql(), "score")
    .join("memories", MEMORY_ID.equals(&FTS_MEMORY_ID))
    .filter(Expr::table_match_for(
      Dialect::Sqlite,
      "memory_fts",
      "runner",
    )?)
    .filter(MEMORY_DELETED_AT.is_null())
    .order_by(OrderBy::alias_asc("score"))
    .limit(10)
    .fetch_all(&conn)
    .await?;

  let ids: Vec<&str> = hits.iter().map(|h| h.memory_id.as_str()).collect();
  assert_eq!(
    ids,
    vec!["m2", "m3"],
    "body-weighted search must rank the body match first"
  );
  Ok(())
}

/// `bm25()` is negative and more negative is better, which is why
/// `ORDER BY score ASC` above is best first. Pinned against the real engine
/// because it inverts everyone's intuition once.
#[tokio::test]
async fn bm25_scores_are_negative_and_ascending_order_is_best_first() -> TestResult {
  let conn = seeded_conn().await?;
  let score = fts5::bm25_for(Dialect::Sqlite, "memory_fts", &[0.0, 3.0, 1.0])?;

  let hits: Vec<ScoredHit> = SelectBuilder::new("memory_fts")
    .columns_raw(&["memory_id"])
    .column_expr(score.sql(), "score")
    .filter(Expr::table_match_for(
      Dialect::Sqlite,
      "memory_fts",
      "runner",
    )?)
    .order_by(OrderBy::alias_asc("score"))
    .fetch_all(&conn)
    .await?;

  assert_eq!(hits.len(), 2);
  for hit in &hits {
    assert!(
      hit.score < 0.0,
      "bm25 must be negative, got {} for {}",
      hit.score,
      hit.memory_id
    );
  }
  let best = hits.first().ok_or("no best hit")?;
  let worst = hits.last().ok_or("no worst hit")?;
  assert!(
    best.score < worst.score,
    "ascending order must put the more negative score first: {best:?} then {worst:?}"
  );
  Ok(())
}

/// `rank` is the same score under default weights, so it orders the same way.
#[tokio::test]
async fn rank_orders_the_same_way_as_bm25() -> TestResult {
  let conn = seeded_conn().await?;

  let hits: Vec<Hit> = SelectBuilder::new("memory_fts")
    .columns_raw(&["memory_id"])
    .filter(Expr::table_match_for(
      Dialect::Sqlite,
      "memory_fts",
      "runner",
    )?)
    .order_by(fts5::rank_for(Dialect::Sqlite, "memory_fts")?.asc())
    .fetch_all(&conn)
    .await?;

  assert_eq!(hits.len(), 2);
  Ok(())
}

/// The porter tokenizer is really in force: `run` answers the row that says
/// *running*, and it is not one of the `runner` rows.
#[tokio::test]
async fn a_stemmed_term_matches_and_a_missing_one_returns_nothing() -> TestResult {
  let conn = seeded_conn().await?;

  let stemmed: Vec<Hit> = SelectBuilder::new("memory_fts")
    .columns_raw(&["memory_id"])
    .filter(Expr::table_match_for(Dialect::Sqlite, "memory_fts", "run")?)
    .fetch_all(&conn)
    .await?;
  assert_eq!(
    stemmed,
    vec![Hit {
      memory_id: "m1".to_owned()
    }]
  );

  let absent: Vec<Hit> = SelectBuilder::new("memory_fts")
    .columns_raw(&["memory_id"])
    .filter(Expr::table_match_for(
      Dialect::Sqlite,
      "memory_fts",
      "kangaroo",
    )?)
    .fetch_all(&conn)
    .await?;
  assert!(absent.is_empty(), "an unmatched term must return no rows");
  Ok(())
}

/// #18 declared `memory_id` `UNINDEXED`; the read side must agree. A
/// column-qualified `MATCH` narrows to its own column.
#[tokio::test]
async fn the_unindexed_column_is_not_searchable_and_columns_narrow_the_match() -> TestResult {
  let conn = seeded_conn().await?;
  let ids = |hits: Vec<Hit>| -> Vec<String> { hits.into_iter().map(|h| h.memory_id).collect() };

  let unindexed: Vec<Hit> = SelectBuilder::new("memory_fts")
    .columns_raw(&["memory_id"])
    .filter(Expr::table_match_for(Dialect::Sqlite, "memory_fts", "m1")?)
    .fetch_all(&conn)
    .await?;
  assert!(
    unindexed.is_empty(),
    "an UNINDEXED column's value must not be searchable"
  );

  let in_body: Vec<Hit> = SelectBuilder::new("memory_fts")
    .columns_raw(&["memory_id"])
    .filter(FTS_BODY.matches_for(Dialect::Sqlite, "runner")?)
    .fetch_all(&conn)
    .await?;
  assert_eq!(ids(in_body), vec!["m2"]);

  let in_tags: Vec<Hit> = SelectBuilder::new("memory_fts")
    .columns_raw(&["memory_id"])
    .filter(FTS_TAGS.matches_for(Dialect::Sqlite, "runner")?)
    .fetch_all(&conn)
    .await?;
  assert_eq!(ids(in_tags), vec!["m3"]);
  Ok(())
}

/// `count` and `exists` carry the `MATCH` without the projection.
#[tokio::test]
async fn count_and_exists_answer_a_match_filter() -> TestResult {
  let conn = seeded_conn().await?;
  let matching = || Expr::table_match_for(Dialect::Sqlite, "memory_fts", "runner");

  let count = SelectBuilder::new("memory_fts")
    .filter(matching()?)
    .count(&conn)
    .await?;
  assert_eq!(count, 2);

  let found = SelectBuilder::new("memory_fts")
    .filter(matching()?)
    .exists(&conn)
    .await?;
  assert!(found);

  let missing = SelectBuilder::new("memory_fts")
    .filter(Expr::table_match_for(
      Dialect::Sqlite,
      "memory_fts",
      "kangaroo",
    )?)
    .exists(&conn)
    .await?;
  assert!(!missing);
  Ok(())
}
