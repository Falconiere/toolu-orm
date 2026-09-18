//! Tests for typed column operations and expression generation.
//!
//! # Public API
//!
//! Tests are organized by concern:
//! - `common_ops` — eq, ne, is_null, is_not_null, in_list, not_in, and/or
//! - `numeric_ops` — gt, lt, lte, gte, between, offset numbering
//! - `text_and_special_ops` — like, json_extract, raw, varchar, date, time
//!
//! # Dialect
//!
//! This binary compiles in the default lane (orm-core with libsql) and again in
//! the postgres lane (orm-core with postgres + libsql), and `to_sql_fragment`
//! renders with `Dialect::CURRENT`, which differs between them. Every assertion
//! on placeholder text therefore names `Dialect::Sqlite` explicitly. Dialect
//! parity for these operators is pinned both ways in
//! `expr_offset_and_nesting_test`; what this binary pins is the typed-column
//! surface — which `Column<T>` gets which operation trait, and what each one
//! renders.
//!
//! The entry file is `main.rs`, not `mod.rs`: Cargo's integration-test
//! auto-discovery only finds `tests/*.rs` and `tests/<dir>/main.rs`, and
//! `orm-core` declares no `[[test]]` targets, so this suite was never built
//! until issue #125. `scripts/check-test-targets.sh` now fails when any test
//! file is unreachable that way.

mod common_ops;
mod numeric_ops;
mod text_and_special_ops;
