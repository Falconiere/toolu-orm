//! Column-marker tagging for Postgres parameter codecs.
//!
//! `From<bool> for Value` stays `0`/`1`, and an integer column keeps that
//! integer. A bind made through `Column<Boolean>` (and the other tagged
//! markers) retags the value so the Postgres encoder can send the column's OID.
//! `LIKE` is left untagged: its argument is a pattern, not a uuid or a timestamp.

use std::any::TypeId;

use crate::column::{Boolean, Json, Jsonb, Numeric, Timestamp, Uuid};
use crate::value::{JsonStorage, Value};

/// Retag `value` for the column marker `T`.
///
/// Markers other than boolean, timestamp, JSON, jsonb, uuid and numeric return
/// `value` unchanged, so an integer column that was given `true` still binds
/// `1`.
#[must_use]
pub fn tag_column_bind<T: 'static>(value: Value) -> Value {
  match codec::<T>() {
    Codec::Boolean => tag_boolean(value),
    Codec::Timestamp => tag_timestamp(value),
    Codec::Json => tag_json(value, JsonStorage::Json),
    Codec::Jsonb => tag_json(value, JsonStorage::Jsonb),
    Codec::Uuid => tag_uuid(value),
    Codec::Numeric => tag_numeric(value),
    Codec::Plain => value,
  }
}

enum Codec {
  Boolean,
  Timestamp,
  Json,
  Jsonb,
  Uuid,
  Numeric,
  Plain,
}

fn codec<T: 'static>() -> Codec {
  let id = TypeId::of::<T>();
  if id == TypeId::of::<Boolean>() {
    Codec::Boolean
  } else if id == TypeId::of::<Timestamp>() {
    Codec::Timestamp
  } else if id == TypeId::of::<Json>() {
    Codec::Json
  } else if id == TypeId::of::<Jsonb>() {
    Codec::Jsonb
  } else if id == TypeId::of::<Uuid>() {
    Codec::Uuid
  } else if id == TypeId::of::<Numeric>() {
    Codec::Numeric
  } else {
    Codec::Plain
  }
}

fn tag_boolean(value: Value) -> Value {
  match value {
    Value::Integer(0) | Value::Boolean(false) => Value::Boolean(false),
    Value::Integer(1) | Value::Boolean(true) => Value::Boolean(true),
    other @ (Value::Null
    | Value::Integer(_)
    | Value::Real(_)
    | Value::Text(_)
    | Value::Blob(_)
    | Value::TimestampEpoch(_)
    | Value::TimestampText(_)
    | Value::Json { .. }
    | Value::Uuid(_)
    | Value::Numeric(_)) => other,
  }
}

fn tag_timestamp(value: Value) -> Value {
  match value {
    Value::Integer(secs) | Value::TimestampEpoch(secs) => Value::TimestampEpoch(secs),
    Value::Text(text) | Value::TimestampText(text) => Value::TimestampText(text),
    other @ (Value::Null
    | Value::Real(_)
    | Value::Blob(_)
    | Value::Boolean(_)
    | Value::Json { .. }
    | Value::Uuid(_)
    | Value::Numeric(_)) => other,
  }
}

fn tag_json(value: Value, storage: JsonStorage) -> Value {
  match value {
    Value::Text(text) | Value::Json { text, .. } => Value::Json { text, storage },
    other @ (Value::Null
    | Value::Integer(_)
    | Value::Real(_)
    | Value::Blob(_)
    | Value::Boolean(_)
    | Value::TimestampEpoch(_)
    | Value::TimestampText(_)
    | Value::Uuid(_)
    | Value::Numeric(_)) => other,
  }
}

fn tag_uuid(value: Value) -> Value {
  match value {
    Value::Text(text) | Value::Uuid(text) => Value::Uuid(text),
    other @ (Value::Null
    | Value::Integer(_)
    | Value::Real(_)
    | Value::Blob(_)
    | Value::Boolean(_)
    | Value::TimestampEpoch(_)
    | Value::TimestampText(_)
    | Value::Json { .. }
    | Value::Numeric(_)) => other,
  }
}

fn tag_numeric(value: Value) -> Value {
  match value {
    Value::Integer(n) => Value::Numeric(n.to_string()),
    Value::Real(f) if f.is_finite() => Value::Numeric(format!("{f}")),
    Value::Real(_) => Value::Numeric("NaN".to_owned()),
    Value::Text(text) | Value::Numeric(text) => Value::Numeric(text),
    other @ (Value::Null
    | Value::Blob(_)
    | Value::Boolean(_)
    | Value::TimestampEpoch(_)
    | Value::TimestampText(_)
    | Value::Json { .. }
    | Value::Uuid(_)) => other,
  }
}
