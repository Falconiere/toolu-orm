//! DDL + seed rows for the live Postgres FTS suite.
//!
//! Wired as `pub mod` via `#[path]` so unused items do not trip `dead_code`.

use toolu_orm_core::column::{Integer, Text};
use toolu_orm_core::query_column::Column;

pub const ID: Column<Text> = Column::new("docs", "id");
pub const BODY: Column<Text> = Column::new("docs", "body");
pub const TAGS: Column<Text> = Column::new("docs", "tags");
pub const DELETED_AT: Column<Integer> = Column::new("docs", "deleted_at");
pub const SEARCH_VECTOR: Column<Text> = Column::new("docs", "search_vector");

pub const DDL: &str = "\
CREATE TABLE docs (\
  id TEXT PRIMARY KEY, \
  body TEXT NOT NULL, \
  tags TEXT NOT NULL, \
  deleted_at BIGINT, \
  search_vector tsvector GENERATED ALWAYS AS (\
    setweight(to_tsvector('english', coalesce(body, '')), 'A') || \
    setweight(to_tsvector('english', coalesce(tags, '')), 'B')\
  ) STORED\
)";

pub const SEED: &[(&str, &str, &str, Option<i64>)] = &[
  ("m1", "the zebrafish keeps running fast", "bio notes", None),
  ("m2", "a marathon runner trains daily", "sport", None),
  ("m3", "shoes and laces", "runner gear", None),
  ("m4", "completely unrelated text", "none", Some(1)),
];
