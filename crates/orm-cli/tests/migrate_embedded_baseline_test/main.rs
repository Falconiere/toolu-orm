//! `mark_applied_embedded` / `mark_applied_through_embedded` /
//! `get_status_embedded` against in-memory libsql: baselining an adopted
//! schema from a compile-time list, status without a migrations directory,
//! unknown and duplicate names, and the first real embedded migrate after
//! adoption.
//!
//! Assertions go through raw libsql so the file compiles in every lane that has
//! libsql, whatever `FromRow` shape orm-core exposes.

#[path = "../fixtures/embedded_list.rs"]
pub mod embedded_list;

mod baseline_tests;
mod rejection_tests;
mod support;
