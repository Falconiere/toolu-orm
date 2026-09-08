//! Dialect-aware DDL generation for migration operations.

mod ddl;
mod gen;
mod postgres;
mod translate;
mod virtual_table;

pub use gen::{generate_sql, generate_sql_for};
