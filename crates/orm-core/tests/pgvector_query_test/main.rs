//! The pgvector distance surface as pure SQL: what `<->` / `<=>` / `<#>`
//! render, and everything they refuse on SQLite.
//!
//! No database here — [pgvector KNN](../../../docs/scenarios/pgvector-knn.md)
//! runs the same constructs against a live server.

mod rejections;
mod rendering;
