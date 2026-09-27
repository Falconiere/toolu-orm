//! Explicit native key-based MERGE, separate from unique-key ON CONFLICT.

mod builder;
mod statement;
mod validate;

pub use builder::{Matched, MergeBuilder, NotMatched};
