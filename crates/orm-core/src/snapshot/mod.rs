//! Schema snapshots for migration generation.

mod extract;
mod serde_compat;
mod types;

pub use types::{ForeignKeyDef, Snapshot, SnapshotEnum, SnapshotMeta, SnapshotTable};
