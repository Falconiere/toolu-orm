//! Real native vector storage, decoding and pre-write validation.
#[cfg(feature = "lancedb")]
mod lance;
#[cfg(feature = "postgres")]
mod postgres;
#[cfg(all(feature = "rusqlite", feature = "sqlite-vec"))]
mod sqlite;
#[cfg(any(
  feature = "postgres",
  feature = "lancedb",
  all(feature = "rusqlite", feature = "sqlite-vec")
))]
pub mod support;
