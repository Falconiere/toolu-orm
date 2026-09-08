//! The FTS5 read surface as pure SQL: what `MATCH` and the auxiliary
//! functions render, and everything they refuse.
//!
//! No database here — [Filters](../../../docs/scenarios/fts5-queries.md) runs
//! the same constructs against real libsql and rusqlite indexes.

mod match_expr;
mod rejections;
mod rendering;
