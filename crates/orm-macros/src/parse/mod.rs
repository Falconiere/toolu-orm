//! Parsing logic for the `#[table]` macro input.
//!
//! # Public API
//!
//! - [`TableInput`], [`ColumnInput`], [`IndexInput`] — parsed representations
//! - [`TypeSpec`] — column type specification
//! - [`parse_struct`] — parse struct fields into column inputs
//! - [`parse_index_attrs`] — extract index attributes from the struct
//! - [`strip_column_attrs`] — remove helper attributes before re-emission
//!
//! # Usage
//!
//! ```ignore
//! let columns = parse_struct(&item_struct)?;
//! let indexes = parse_index_attrs(&mut item_struct)?;
//! ```

mod column_flags;
mod column_parsing;
mod index_parsing;
mod pk_validation;
mod primary_key_parsing;
pub mod relation_parsing;
mod table_attrs;
mod vec0_column;

pub use column_parsing::{parse_struct, strip_column_attrs, ColumnInput, TypeSpec};
pub use index_parsing::{parse_index_attrs, IndexInput, TableInput};
pub use pk_validation::{reject_mixed_primary_keys, validate_autoincrement};
pub use primary_key_parsing::parse_primary_key_attr;
pub use table_attrs::parse_table_attrs;
