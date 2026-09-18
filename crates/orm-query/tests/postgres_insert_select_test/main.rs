//! `INSERT … SELECT` against live Postgres: a **schema**-qualified target and
//! source, the conflict modes, and the unwrapped explicit clause.
//!
//! Postgres spells the qualifier exactly as SQLite does and resolves it
//! differently — `"s"."t"` is a namespace here, an attachment there. Nothing
//! translates between them, which is what these tests hold in place.
//!
//! Needs the live server from `docker-compose.test.yaml`
//! (`TEST_DB_PORT=5434`). A missing server fails the tests; it never skips
//! them.

#[path = "../fixtures/pg_insert_select_db.rs"]
pub mod db;
#[path = "../fixtures/insert_select_schema.rs"]
pub mod schema;

mod copy;
