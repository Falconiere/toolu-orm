//! The same write routine on real selected connections.
//!
//! CI runs this binary on the mixed-driver lane in `.github/workflows/ci.yml`
//! (`postgres,rusqlite,sqlite-vec`) plus each single-driver lane below.
#[cfg(feature = "lancedb")]
mod lance;
#[cfg(feature = "libsql")]
mod libsql;
#[cfg(feature = "postgres")]
mod postgres;
#[cfg(feature = "rusqlite")]
mod sqlite;
#[cfg(any(
  feature = "postgres",
  feature = "rusqlite",
  feature = "libsql",
  feature = "lancedb"
))]
mod support;
