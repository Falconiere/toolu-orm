//! The dialect-neutral half of the scalar-expression scenarios against the
//! live Postgres server (postgres lane), proving `$N` parity with SQLite.
//!
//! `datetime(...)` is deliberately absent: it is a SQLite function, and this
//! crate does not translate function names between dialects. Its Postgres
//! counterpart is a cast or `to_timestamp`, which is the caller's choice.

#[path = "../fixtures/pg_memories_db.rs"]
pub mod db;
#[path = "../fixtures/memories_seed.rs"]
pub mod seed;

mod arithmetic;
mod nulls;
mod projections;
mod support;
mod wildcards;
