//! Dialect-aware DDL generation for migration operations.

mod ddl;
mod foreign_key;
mod fts5_triggers;
mod gen;
mod operation_sql;
mod policy;
mod postgres;
mod rebuild;
mod translate;
mod virtual_table;

pub use gen::{generate_sql, generate_sql_for};
