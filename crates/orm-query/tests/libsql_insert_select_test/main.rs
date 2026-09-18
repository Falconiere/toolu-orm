//! `INSERT … SELECT` against a real in-memory libsql database (libsql-only
//! lane, asynchronous): the same statement shapes the rusqlite suite proves
//! across an `ATTACH`, driven through the async executor.
//!
//! libsql has no `SqliteMaintenance`, so the cross-file copy stays in the
//! rusqlite suite; here `main` is the qualifier under test.

#[path = "../fixtures/libsql_insert_select_db.rs"]
pub mod db;
#[path = "../fixtures/insert_select_schema.rs"]
pub mod schema;

mod copy;
