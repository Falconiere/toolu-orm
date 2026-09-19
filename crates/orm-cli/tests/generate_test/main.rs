//! `run_generate` against a temporary migrations directory: the journal and
//! snapshot it writes, the incremental diff, the no-change case, and the DDL
//! each dialect produces.

mod dialect_tests;
mod journal_tests;
mod support;
