//! Issue #108's upsert acceptance against a real in-memory SQLite database
//! (rusqlite-only lane, synchronous): partial-field preservation, counter
//! increments, `DO NOTHING`, foreign-key safety contrasted with
//! `INSERT OR REPLACE`, generated ids through `RETURNING`, and a
//! database-clock expression value.

#[path = "../fixtures/rusqlite_upsert_db.rs"]
pub mod db;
#[path = "../fixtures/upsert_schema.rs"]
pub mod schema;

mod clock;
mod counters;
mod generated_ids;
mod references;
mod support;
