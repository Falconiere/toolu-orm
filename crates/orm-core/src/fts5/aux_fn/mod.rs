//! The FTS5 auxiliary functions as selectable and orderable SQL.
//!
//! `bm25` and `rank` score a match; `snippet` and `highlight` mark it up. All
//! four render straight to text — FTS5 refuses bound parameters as auxiliary
//! arguments — and all four refuse [`crate::dialect::Dialect::Postgres`].

mod call;
mod score;
mod text;

pub use call::Fts5Fn;
pub use score::{bm25, bm25_for, rank, rank_for};
pub use text::{column_index, highlight, highlight_for, snippet, snippet_for, Highlight, Snippet};
