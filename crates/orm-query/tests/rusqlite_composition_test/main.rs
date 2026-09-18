//! The query-composition scenarios executed against a real in-memory rusqlite
//! database (rusqlite-only lane): a recursive walk over a cyclic graph, set
//! operations, NULL-sensitive subquery predicates, a correlated scalar
//! subquery, set-based DML, and SQLite's table-valued `FROM` sources.

#[path = "../fixtures/rusqlite_composition_db.rs"]
pub mod db;
#[path = "../fixtures/composition_json_walk.rs"]
pub mod json_walk;
#[path = "../fixtures/composition_queries.rs"]
pub mod queries;
#[path = "../fixtures/composition_seed.rs"]
pub mod seed;

mod null_sensitive;
mod recursive_walk;
mod scalar_subquery;
mod set_based_dml;
mod set_ops;
mod table_functions;
