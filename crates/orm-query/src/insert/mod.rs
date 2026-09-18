//! INSERT query builder with conflict handling.
//!
//! # Public API
//!
//! - [`InsertBuilder`] — fluent builder for INSERT statements
//! - [`OnConflict`] — an explicit `ON CONFLICT (…) DO NOTHING | DO UPDATE SET …`
//!   clause, rendered identically on SQLite and Postgres

mod builder;
mod conflict;
mod on_conflict;

use crate::where_clause::cfg_single_backend;

cfg_single_backend! {
  mod fetch;
}

pub use builder::InsertBuilder;
pub use on_conflict::OnConflict;
