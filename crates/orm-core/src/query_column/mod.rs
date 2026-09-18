//! `Column<T>` and the typed column operations that build expressions from it.

mod column;
mod common_ops;
mod fts5_ops;
mod numeric_ops;
mod shared_ops;
mod text_ops;
mod vec0_ops;

pub use column::{Column, ColumnRef};
pub use common_ops::CommonOps;
pub use fts5_ops::Fts5Ops;
pub use numeric_ops::NumericOps;
pub use shared_ops::SharedOps;
pub use text_ops::TextOps;
pub use vec0_ops::Vec0Ops;
