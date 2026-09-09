//! Postgres full-text query surface: `@@`, `to_tsquery` / `plainto_tsquery` /
//! `websearch_to_tsquery`, and `ts_rank`.
//!
//! Separate from [`crate::fts5`]: SQLite FTS5 `MATCH` / `bm25` are a different
//! model and are never translated here. Every constructor requires
//! [`crate::dialect::Dialect::Postgres`] and refuses SQLite at construction
//! time so no SQLite statement can carry these forms.

mod document;
mod literal;
mod matches;
mod rank;

pub use document::{column, column_for, to_tsvector, to_tsvector_for, PgTsDocument};
pub use rank::{
  ts_rank_plainto_tsquery, ts_rank_plainto_tsquery_for, ts_rank_tsquery, ts_rank_tsquery_for,
  ts_rank_websearch_to_tsquery, ts_rank_websearch_to_tsquery_for, PgFtsFn,
};
