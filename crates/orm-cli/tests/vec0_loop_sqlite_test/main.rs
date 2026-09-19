//! Generate → migrate against in-memory libsql for a `vec0` table: without
//! `sqlite-vec` the apply fails as [`MigrateError::MissingExtension`], an
//! identical regenerate is a no-op, and a dimension / metric change is refused
//! before a migration file is written. An FTS5-only migration on the same
//! driver still applies, so the mapping is not blanket.

mod generate_tests;
mod migrate_tests;
mod support;
