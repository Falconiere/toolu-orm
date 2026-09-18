//! Issue #108's upsert acceptance against a real in-memory libsql database
//! (libsql-only lane, asynchronous): the same scenarios the rusqlite suite
//! proves, driven through the async executor.

#[path = "../fixtures/libsql_upsert_db.rs"]
pub mod db;
#[path = "../fixtures/upsert_schema.rs"]
pub mod schema;

mod clock;
mod counters;
mod generated_ids;
mod references;
mod support;
