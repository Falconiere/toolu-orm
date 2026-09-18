//! The rusqlite grouping scenarios, executed against a real in-memory libsql
//! database (libsql-only lane): deduplication applied before pagination,
//! grouped reports over one and two keys, every aggregate, ordering by an
//! aggregate's alias, HAVING with bound parameters, empty result sets,
//! grouped-count semantics, a grouping key from a joined aliased relation, and
//! the raw escape hatches.

#[path = "../fixtures/libsql_grouping_db.rs"]
pub mod db;
#[path = "../fixtures/grouping_seed.rs"]
pub mod seed;

mod aggregates;
mod counting;
mod distinct;
mod grouped_reports;
mod having;
mod qualified;
mod support;
