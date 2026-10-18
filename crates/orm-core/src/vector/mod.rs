//! Dimension-checked finite f32 vectors with backend-specific codecs.
#[cfg(feature = "postgres")]
mod postgres;
#[cfg(feature = "rusqlite")]
mod sqlite;
mod typed;
mod value;

pub use typed::Vector;
pub use value::VectorValue;
