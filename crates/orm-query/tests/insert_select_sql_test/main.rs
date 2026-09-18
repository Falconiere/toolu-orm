//! `INSERT … SELECT` with explicit target columns, a database-qualified target
//! and a conflict mode (issue #114), rendered per dialect.
//!
//! Driver-free, so this binary runs in the default lane *and* in the postgres
//! lane, where `Dialect::CURRENT` is Postgres. Every assertion therefore names
//! its dialect with `to_sql_for`; none calls `to_sql()`.
//!
//! The executed counterparts — including a real `ATTACH`-based copy — are
//! `{rusqlite,libsql,postgres}_insert_select_test`.

mod bind_order;
mod conflict;
mod fixtures;
mod rendering;
