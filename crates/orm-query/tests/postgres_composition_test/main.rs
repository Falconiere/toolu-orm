//! The query-composition scenarios executed against the live Postgres of
//! `docker-compose.test.yaml` (postgres lane): a recursive walk over a cyclic
//! graph, set operations, NULL-sensitive subquery predicates, a correlated
//! scalar subquery, set-based DML, and a table-valued `FROM` source.
//!
//! `json_each` and `pragma_table_info` are SQLite-only *functions*; the
//! mechanism is not, so the table-valued source here is
//! `regexp_split_to_table($1, $2)`. Every test owns a schema, and a missing
//! server fails the test rather than skipping it.

#[path = "../fixtures/pg_composition_db.rs"]
pub mod db;
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
