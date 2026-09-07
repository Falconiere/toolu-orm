//! Code generation (expansion) for the `#[table]` macro.
//!
//! # Public API
//!
//! - [`expand`] — generate `TableSchema` impl
//! - [`expand_builder_methods`] — generate select/insert/update/delete factory methods
//! - [`expand_columns_module`] — generate companion `mod` with typed column constants
//!
//! # Usage
//!
//! ```ignore
//! let schema_tokens = expand(&table_input);
//! let builder_tokens = expand_builder_methods(&table_input);
//! let columns_tokens = expand_columns_module(&table_input);
//! ```

mod columns_expansion;
mod relational_expansion;
mod schema_expansion;

pub use columns_expansion::expand_columns_module;
pub use relational_expansion::expand_relational;
pub use schema_expansion::{expand, expand_builder_methods};
