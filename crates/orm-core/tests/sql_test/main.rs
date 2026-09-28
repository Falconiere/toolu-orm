//! SQL generation tests for all DDL operations.

#[path = "../fixtures/composite_fk_schema.rs"]
pub mod composite_fk_schema;

mod column_add_drop_alter_sqlite;
mod column_create_constraints;
mod column_helpers;
mod column_postgres_alter;
mod composite_fk_operations;
mod constraint_operations;
mod enum_operations;
mod fk_operations;
mod fts5_recreate;
mod index_operations;
mod primary_key_operations;
mod rename_operations;
mod table_operations;
