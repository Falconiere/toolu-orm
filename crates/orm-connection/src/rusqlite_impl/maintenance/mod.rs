//! Typed SQLite maintenance and inspection on a connection you own.
//!
//! SQLite's administration statements are not DML, so no query builder can
//! reach them: `VACUUM`, `ATTACH`, `DETACH` and `PRAGMA` name schemas and files
//! rather than tables and columns. This module is the typed replacement for
//! hand-building them, and it works on a **borrowed** `&rusqlite::Connection` —
//! it never opens, wraps, or takes ownership of anything, so the caller keeps
//! its connection and its transactions.
//!
//! Start at [`SqliteMaintenance`]. Everything here is `rusqlite`-only, and none
//! of it depends on which other driver features are enabled.

mod attached;
mod error;
mod integrity;
mod ops;
mod pragma_reads;
mod schema_name;
mod storage;

pub use attached::AttachedDatabase;
pub use error::MaintenanceError;
pub use integrity::IntegrityReport;
pub use ops::SqliteMaintenance;
pub use storage::StorageStats;
