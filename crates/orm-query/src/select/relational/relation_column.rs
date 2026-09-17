//! One projected relation column and how it travels through the JSON transport.

/// A column selected from a relation's target table.
///
/// JSON has no binary type, so a column declared with [`RelationColumn::binary`]
/// is hex-encoded in the generated SQL and decoded back to exact bytes by
/// [`decode_relation_value`](super::RelationalSelectBuilder::decode_relation_value).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationColumn {
  name: String,
  binary: bool,
}

impl RelationColumn {
  /// A column projected as-is: text, integer, real or null.
  pub fn new(name: &str) -> Self {
    Self {
      name: name.to_owned(),
      binary: false,
    }
  }

  /// A binary column (SQLite `BLOB`, Postgres `bytea`).
  ///
  /// Hex text on the wire, a JSON array of byte values after decoding, so a
  /// `Vec<u8>` / `Option<Vec<u8>>` field deserializes to the exact stored bytes.
  pub fn binary(name: &str) -> Self {
    Self {
      name: name.to_owned(),
      binary: true,
    }
  }

  /// The column name on the target table.
  pub fn name(&self) -> &str {
    &self.name
  }

  /// Whether this column is hex-encoded in SQL and decoded back to bytes.
  pub fn is_binary(&self) -> bool {
    self.binary
  }
}

impl From<&str> for RelationColumn {
  fn from(name: &str) -> Self {
    Self::new(name)
  }
}
