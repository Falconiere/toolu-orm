//! Dialect-aware default value translation (e.g. booleans, UUIDs).

use crate::dialect::Dialect;

/// Translate a column default only when its dialect needs it.
pub(crate) fn translate_default(default: &str, dialect: Dialect) -> String {
  match dialect {
    Dialect::Sqlite => match default.trim() {
      "true" => "1".to_owned(),
      "false" => "0".to_owned(),
      _ => default.to_owned(),
    },
    Dialect::Postgres => dialect.map_default(default),
    Dialect::Lance => default.to_owned(),
  }
}
