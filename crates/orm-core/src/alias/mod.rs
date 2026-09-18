//! Table aliases and alias-aware column references.
//!
//! # Public API
//!
//! - [`TableRef`] — a table in a `FROM` / `JOIN` slot, with an optional alias
//! - [`AliasedColumn`] — one column of such a table, qualified by the alias
//! - [`QualifiedColumn`] — what a `Column<T>` and an `AliasedColumn<T>` share

mod aliased_column;
mod compare;
mod qualified;
mod quoting;
mod table_ref;

pub use aliased_column::AliasedColumn;
pub use qualified::QualifiedColumn;
pub use table_ref::TableRef;
