//! Tests for the `#[table]` macro across all features.
//!
//! # Public API
//!
//! Tests are organized by category:
//! - `schema_basics` — core table/column generation
//! - `column_types_and_fk` — new types, FK actions, strict mode
//! - `enums_and_indexes` — ColumnEnum derive, index parsing
//! - `views` — view macros (omit/pick)
//! - `builder_methods` — companion modules, factory methods, serde

pub mod builder_methods;
pub mod column_types_and_fk;
pub mod enums_and_indexes;
pub mod primary_keys;
pub mod schema_basics;
pub mod views;
