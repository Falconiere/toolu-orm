//! Integration tests for the ORM query builders against an in-memory libsql database.
//!
//! # Public API
//!
//! Tests are organized by query type:
//! - `select_queries` — fetch_all, fetch_one, fetch_optional, filters, order, pagination
//! - `mutation_queries` — insert, update, delete, count, exists, transactions, dynamic filters

mod mutation_queries;
mod select_queries;
mod test_helpers;

pub use test_helpers::integration_users;
pub(crate) use test_helpers::setup;
pub use test_helpers::IntegrationUser;
