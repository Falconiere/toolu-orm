//! The Postgres FTS read surface as pure SQL: what `@@` / `ts_rank` render,
//! and everything they refuse on SQLite.
//!
//! No database here — [Postgres FTS queries](../../../docs/scenarios/postgres-fts-queries.md)
//! runs the same constructs against a live server.

mod match_expr;
mod rejections;
mod rendering;
