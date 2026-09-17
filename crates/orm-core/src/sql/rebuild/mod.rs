//! SQLite table rebuilds: one coordinated rebuild per changed table.
//!
//! Renaming the old table out of the way first rewrites every child table's
//! `REFERENCES` clause (SQLite ≥ 3.26 does that whether or not foreign keys
//! are enforced), and dropping it then runs the child's `ON DELETE` action.
//! [`plan`] and [`emit`] implement the order SQLite documents instead: create
//! a staging table, copy, drop the old one, rename the staging table into
//! place. See <https://sqlite.org/lang_altertable.html> §8.

mod emit;
mod plan;

pub(crate) use emit::rebuild_sql;
pub(crate) use plan::{plan_sqlite_rebuilds, SqliteStep};
