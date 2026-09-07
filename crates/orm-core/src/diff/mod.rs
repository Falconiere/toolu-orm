//! Schema diffing for migration generation.

mod column;
mod engine;
mod enums;
mod fk;
mod operation;

pub use engine::{diff, diff_with_resolver};
pub use enums::diff_enums;
pub use fk::diff_foreign_keys;
pub use operation::{ColumnChange, Operation};
