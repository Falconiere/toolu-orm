//! The scalar-expression scenarios against in-memory libsql (libsql-only
//! lane): the same cases `rusqlite_scalar_expr_test` runs, asserted on the
//! same seeded rows, so both SQLite drivers are held to one result.

#[path = "../fixtures/libsql_memories_db.rs"]
pub mod db;
#[path = "../fixtures/memories_seed.rs"]
pub mod seed;

mod arithmetic;
mod nulls;
mod projections;
mod support;
mod timestamps;
mod wildcards;
