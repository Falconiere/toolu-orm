//! The grouping scenarios against a live Postgres (postgres lane).
//!
//! Three things only this lane can prove: that the derived table a grouped
//! `count()` builds is accepted (Postgres *requires* the alias SQLite merely
//! tolerates), that a `DISTINCT` query's `ORDER BY` term is accepted because
//! it is textually a select-list item, and that a `HAVING` bind arrives as
//! `$N`.
//!
//! `SUM` and `AVG` over a `bigint` return `numeric` on Postgres, which the row
//! decoders do not map to a Rust scalar; those two aggregates are proven live
//! on both SQLite drivers instead, and rendered for both dialects in
//! `grouping_sql_test`.

#[path = "../fixtures/pg_grouping_db.rs"]
pub mod db;
#[path = "../fixtures/grouping_seed.rs"]
pub mod seed;

mod aggregates;
mod counting;
mod distinct;
mod grouped_reports;
mod having;
mod qualified;
mod support;
