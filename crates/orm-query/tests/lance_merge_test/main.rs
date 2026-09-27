//! Explicit key-based MERGE on the checksum-pinned real Lance extension.

mod behavior;
#[path = "../fixtures/invalid_merge.rs"]
pub mod invalid_merge;
#[path = "../fixtures/merge.rs"]
pub mod merge;
mod quoted;
mod raw_control;
mod support;
