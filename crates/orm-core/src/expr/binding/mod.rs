//! Reusable bound parameters: a value bound once and referenced from every
//! predicate built out of the same handle.
//!
//! - [`SharedBind`] / [`SharedBindList`] — the handles
//! - [`BoundParams`] — the statement-wide buffer that records what each handle
//!   bound, so the second occurrence emits an index instead of a value

mod handle;
mod params;
mod source;

pub use handle::{SharedBind, SharedBindList};
pub use params::BoundParams;

pub(crate) use source::{BindSource, ListSource};
