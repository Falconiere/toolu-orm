// Re-export backend crates so downstream dependents (e.g. orm-cli) can
// reference row types without declaring their own direct dependency.
#[cfg(feature = "libsql")]
pub use libsql;
#[cfg(feature = "rusqlite")]
pub use rusqlite;
#[cfg(feature = "postgres")]
pub use tokio_postgres;

// `#[table]`'s view structs derive Serialize/Deserialize and `Relational`
// names `serde_json::Value`, so the expansions reach both through here rather
// than through the consumer's extern prelude.
pub use {serde, serde_json};

pub mod column;
pub mod dialect;
pub mod diff;
pub mod error;
pub mod expr;
pub mod fts5;
pub mod index;
pub mod journal;
pub mod ordering;
pub mod query_column;
pub mod relation;
pub mod relational_row;
pub mod rename;
pub mod row;
pub mod schema;
pub mod snapshot;
pub mod sql;
pub mod table;
pub mod value;
