//! SQLite storage form of a [`Value`](crate::value::Value).
//!
//! Postgres-tagged payloads become the integer or text SQLite already stores.
//! `From<bool>` stays an integer; only a tagged boolean takes this path.

use super::Value;

pub(super) enum SqliteForm<'a> {
  Null,
  Integer(i64),
  Real(f64),
  Text(&'a str),
  Blob(&'a [u8]),
}

impl Value {
  pub(super) fn sqlite_form(&self) -> SqliteForm<'_> {
    match self {
      Self::Null => SqliteForm::Null,
      Self::Integer(n) | Self::TimestampEpoch(n) => SqliteForm::Integer(*n),
      Self::Boolean(true) => SqliteForm::Integer(1),
      Self::Boolean(false) => SqliteForm::Integer(0),
      Self::Real(f) => SqliteForm::Real(*f),
      Self::Text(s) | Self::TimestampText(s) | Self::Uuid(s) | Self::Numeric(s) => {
        SqliteForm::Text(s)
      },
      Self::Blob(b) => SqliteForm::Blob(b),
      Self::Json { text, .. } => SqliteForm::Text(text),
    }
  }

  /// The integer or text SQLite binds for this value.
  ///
  /// Untyped values are unchanged. A tagged boolean is `0` or `1`, a timestamp
  /// epoch stays an integer, and timestamp text, JSON, UUID and numeric become
  /// text. Postgres encoding does not use this form.
  #[must_use]
  pub fn sqlite_stored(&self) -> Self {
    match self.sqlite_form() {
      SqliteForm::Null => Self::Null,
      SqliteForm::Integer(n) => Self::Integer(n),
      SqliteForm::Real(f) => Self::Real(f),
      SqliteForm::Text(s) => Self::Text(s.to_owned()),
      SqliteForm::Blob(b) => Self::Blob(b.to_owned()),
    }
  }
}
