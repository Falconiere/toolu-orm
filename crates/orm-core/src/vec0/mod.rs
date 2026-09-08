//! `sqlite-vec` `vec0` vector tables on top of the general
//! [`crate::table::TableKind::Virtual`] mechanism.
//!
//! [`Vec0Table`] renders the module arguments — the vector columns with their
//! element type, dimension and `distance_metric`, plus the primary key,
//! partition keys, metadata and auxiliary columns — so callers never
//! hand-write them. The `#[vec0_table]` proc macro expands into this builder,
//! so a macro-declared table and a hand-built one cannot drift apart.
//!
//! Unlike FTS5, `vec0` is not part of SQLite: `sqlite-vec` has to be
//! registered on the connection before any `vec0` DDL is prepared, which means
//! before `run_migrate`. Without it the migration fails with
//! `MigrateError::MissingExtension`.
//!
//! The KNN read surface — [`distance`], [`k_eq`], and
//! [`crate::query_column::Vec0Ops`] — builds the `MATCH` / hidden `k` /
//! synthesised `distance` form. Executing that SQL also needs the extension.

mod builder;
mod distance;
mod ident;
mod k_eq;
pub(crate) mod require;
mod role;
mod types;

pub use builder::{Vec0Table, VEC0_MODULE};
pub use distance::{distance, distance_for, Vec0Distance};
pub use ident::is_vec0_ident;
pub use k_eq::{k_eq, k_eq_for};
pub use types::{DistanceMetric, Vec0AuxiliaryType, Vec0KeyType, Vec0MetadataType};
