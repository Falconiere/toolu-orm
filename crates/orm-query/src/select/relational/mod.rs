//! Relational SELECT: lateral joins (Postgres) or correlated subqueries (SQLite).
//!
//! # Public API
//!
//! - [`RelationalSelectBuilder`], [`RelationConfig`]

mod config;
mod postgres_sql;
mod sqlite_sql;

pub use config::{RelationConfig, RelationalSelectBuilder};
