//! INSERT query builder with conflict handling.
//!
//! # Public API
//!
//! - [`InsertBuilder`] — fluent builder for INSERT statements
//! - [`OnConflict`] — `ON CONFLICT (…) [WHERE …] DO NOTHING | DO UPDATE SET … [WHERE …]`,
//!   rendered identically on SQLite and Postgres
//! - `InsertBuilder::select` — `INSERT INTO "t" ("a", "b") SELECT …`, a
//!   set-based copy whose rows are never decoded into Rust

mod builder;
mod conflict;
mod on_conflict;
mod rows;
mod statement;

use crate::where_clause::cfg_single_backend;

cfg_single_backend! {
  mod fetch;
}

pub use builder::InsertBuilder;
pub use on_conflict::OnConflict;
