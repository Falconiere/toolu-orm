//! FTS5 full-text tables on top of the general [`crate::table::TableKind::Virtual`]
//! mechanism.
//!
//! [`Fts5Table`] renders the module arguments — quoted column names,
//! `UNINDEXED` markers, and the `key = 'value'` options — so callers never
//! hand-write them. The `#[fts5_table]` proc macro expands into this builder,
//! so a macro-declared table and a hand-built one cannot drift apart.

mod builder;
mod options;

pub use builder::{Fts5Table, FTS5_MODULE};
pub use options::Fts5Options;
