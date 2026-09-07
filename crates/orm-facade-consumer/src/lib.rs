//! A consumer whose only dependency is `toolu-orm`.
//!
//! This crate ships nothing. It exists so the test suites in `tests/` compile
//! under the same constraint an external user has: `toolu-orm` is the single
//! entry in `[dependencies]`, so `toolu_orm_core`, `toolu_orm_query`, `serde`
//! and `serde_json` are absent from the extern prelude and every path the
//! macros emit has to resolve on its own.
//!
//! `crates/orm/tests/facade_test.rs` cannot prove this: the `toolu-orm`
//! package depends on the four library crates directly.
