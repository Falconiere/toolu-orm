//! The same write routine on real selected connections.
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
