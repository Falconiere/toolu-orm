//! Schema snapshots for migration generation.

mod extract;
mod serde_compat;
mod types;

pub(crate) use extract::table_level_foreign_keys;
pub use types::{ForeignKeyDef, Snapshot, SnapshotEnum, SnapshotMeta, SnapshotTable};
