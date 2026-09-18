//! Scalar expressions executed against a real in-memory SQLite database
//! (rusqlite-only lane): literal wildcards through `LIKE … ESCAPE`,
//! mixed-precision timestamps through `datetime()`, `COALESCE` over NULLs,
//! an arithmetic self-update, `CASE`/concatenation, and binds landing in
//! SELECT, SET and WHERE of one statement in the right order.

#[path = "../fixtures/rusqlite_memories_db.rs"]
pub mod db;
#[path = "../fixtures/memories_seed.rs"]
pub mod seed;

mod arithmetic;
mod nulls;
mod projections;
mod support;
mod timestamps;
mod wildcards;
