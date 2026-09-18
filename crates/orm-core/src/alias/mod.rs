//! Table references: database qualifiers, aliases, and alias-aware columns.
//!
//! # Public API
//!
//! - [`TableRef`] — a table, CTE or table-valued call in a `FROM` / `JOIN` /
//!   `INSERT INTO` slot, with an optional database qualifier and alias
//! - [`AliasedColumn`] — one column of such a table, qualified by the alias
//! - [`QualifiedColumn`] — what a `Column<T>` and an `AliasedColumn<T>` share
//! - [`quote_ident`] — a double-quoted identifier, embedded quotes doubled

mod aliased_column;
mod compare;
mod qualified;
mod quoting;
mod table_function;
mod table_ref;

pub use aliased_column::AliasedColumn;
pub use qualified::QualifiedColumn;
pub use quoting::quote_ident;
pub use table_ref::TableRef;
