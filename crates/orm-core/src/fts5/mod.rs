//! FTS5 full-text tables on top of the general [`crate::table::TableKind::Virtual`]
//! mechanism, and the query surface that reads them.
//!
//! [`Fts5Table`] renders the module arguments — quoted column names,
//! `UNINDEXED` markers, and the `key = 'value'` options — so callers never
//! hand-write them. The `#[fts5_table]` proc macro expands into this builder,
//! so a macro-declared table and a hand-built one cannot drift apart.
//!
//! On the read side, [`bm25`], [`rank`], [`snippet`] and [`highlight`] build
//! the auxiliary-function calls, and [`crate::expr::Expr::table_match`] plus
//! [`crate::query_column::Fts5Ops`] build the `MATCH` operator that consults
//! the index. Every one of them is SQLite-only and says so instead of emitting
//! SQL Postgres cannot run.

mod aux_fn;
mod builder;
pub(crate) mod literal;
mod options;

pub use aux_fn::{
  bm25, bm25_for, column_index, highlight, highlight_for, rank, rank_for, snippet, snippet_for,
  Fts5Fn, Highlight, Snippet,
};
pub use builder::{Fts5Table, FTS5_MODULE};
pub use options::Fts5Options;
