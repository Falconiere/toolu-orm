//! Issue #114's acceptance against two **real on-disk SQLite databases** joined
//! by `ATTACH`: a set-based copy carrying NULLs, blobs and a projected default
//! for a column the older store does not have, plus the conflict modes and a
//! database-qualified catalogue read.
//!
//! Nothing here is mocked and nothing substitutes an in-memory database for the
//! attached file — `ATTACH DATABASE` names a filename, so it cannot.
//! The ATTACH lifecycle itself is `SqliteMaintenance::attach_database` from
//! issue #115, used as-is; this suite only relies on it.

#[path = "../fixtures/attached_copy_db.rs"]
pub mod db;
#[path = "../fixtures/temp_db_dir.rs"]
pub mod temp_db_dir;

mod catalogue;
mod conflict;
mod copy;
mod support;
