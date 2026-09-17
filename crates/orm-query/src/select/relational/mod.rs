//! Relational SELECT: lateral joins (Postgres) or correlated subqueries (SQLite).
//!
//! # Public API
//!
//! - [`RelationalSelectBuilder`], [`RelationConfig`], [`RelationColumn`]

mod config;
mod decode;
mod identifier;
mod postgres_sql;
mod relation_column;
mod sqlite_sql;

pub use config::{RelationConfig, RelationalSelectBuilder};
pub use relation_column::RelationColumn;
