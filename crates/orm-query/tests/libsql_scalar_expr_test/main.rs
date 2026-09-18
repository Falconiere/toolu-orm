//! The rusqlite scalar-expression scenarios against in-memory libsql
//! (libsql-only lane), so both SQLite drivers are held to the same results.

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
