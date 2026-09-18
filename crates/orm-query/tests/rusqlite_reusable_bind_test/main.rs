//! The reusable-binding scenarios executed against a real in-memory rusqlite
//! database (rusqlite-only lane): the issue's own 16,381-path co-change
//! lookup, sharing across clause and statement boundaries, and the mutation
//! builders.

#[path = "../fixtures/rusqlite_reusable_bind_db.rs"]
pub mod db;
#[path = "../fixtures/reusable_bind_seed.rs"]
pub mod seed;

mod clauses;
mod co_change;
mod nesting;
