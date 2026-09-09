//! DDL + seed rows for the live pgvector KNN suite.
//!
//! Wired as `pub mod` via `#[path]` so unused items do not trip `dead_code`.
//! Requires `CREATE EXTENSION vector` on the server (pgvector image).

use toolu_orm_core::column::{Text, Vector};
use toolu_orm_core::query_column::Column;

pub const ID: Column<Text> = Column::new("items", "id");
pub const EMBEDDING: Column<Vector> = Column::new("items", "embedding");

/// Table DDL after `CREATE EXTENSION vector`. Dim-3 for readable fixtures.
pub const DDL: &str = "\
CREATE TABLE items (\
  id TEXT PRIMARY KEY, \
  embedding vector(3) NOT NULL\
)";

/// (id, pgvector text literal without cast).
pub const SEED: &[(&str, &str)] = &[
  ("origin", "[0,0,0]"),
  ("near", "[1,0,0]"),
  ("far", "[10,0,0]"),
];
