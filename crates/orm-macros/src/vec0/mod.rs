//! `#[vec0_table]`: sqlite-vec `vec0` virtual tables declared as structs.
//!
//! Expands into a `TableSchema` impl built with
//! `toolu_orm_core::vec0::Vec0Table`, so the module arguments are rendered in
//! exactly one place and a macro-declared table cannot drift from a
//! hand-built one.
//!
//! `vec0` parses its own constructor with a scanner that has no quoting, so
//! every identifier is checked here at expansion time, against the same
//! `is_vec0_ident` the builder uses.

mod attrs;
mod columns;
mod entry;
mod expand;
mod types;

pub use entry::expand_vec0_table;
