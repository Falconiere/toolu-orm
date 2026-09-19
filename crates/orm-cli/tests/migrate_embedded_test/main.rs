//! `run_migrate_embedded` against in-memory libsql: applying a compile-time
//! list, re-running it, a tampered body, a repeated name, the empty list, a
//! failing statement, slice order beating name order, the database-free hash
//! check, and interchange with the directory runner in both directions.
//!
//! Assertions go through raw libsql so the file compiles in every lane that has
//! libsql, whatever `FromRow` shape orm-core exposes.

#[path = "../fixtures/baseline_dir.rs"]
pub mod baseline_dir;
#[path = "../fixtures/embedded_list.rs"]
pub mod embedded_list;

mod apply_tests;
mod interchange_tests;
mod rejection_tests;
mod support;
