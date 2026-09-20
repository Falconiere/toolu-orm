//! Postgres parameter encoding for [`Value`](crate::value::Value).
//!
//! Untyped null, integer, real, text and blob keep their old OIDs. Tagged
//! boolean, timestamp, JSON, UUID and numeric payloads use the Postgres type
//! the column declared. A value that cannot be encoded fails here, before
//! `tokio_postgres` is asked to send it.

use postgres_types::ToSql;

use crate::error::DbCoreError;

use super::{JsonStorage, Value};

#[path = "value_pg_time.rs"]
mod time_codec;

use time_codec::{epoch_secs_to_pg_micros, rfc3339_to_pg_micros};

/// Type-agnostic SQL NULL accepted by any Postgres column type.
#[derive(Debug)]
struct PgNull;

impl postgres_types::ToSql for PgNull {
  fn to_sql(
    &self,
    _ty: &postgres_types::Type,
    _out: &mut bytes::BytesMut,
  ) -> Result<postgres_types::IsNull, Box<dyn std::error::Error + Send + Sync>> {
    Ok(postgres_types::IsNull::Yes)
  }

  fn accepts(_ty: &postgres_types::Type) -> bool {
    true
  }

  postgres_types::to_sql_checked!();
}

/// Text value that also accepts Postgres UUID columns.
///
/// When the target column is UUID, parses the string and writes binary format.
/// For all other text-compatible types, delegates to the standard `String` impl.
#[derive(Debug)]
struct FlexibleText(String);

impl postgres_types::ToSql for FlexibleText {
  fn to_sql(
    &self,
    ty: &postgres_types::Type,
    out: &mut bytes::BytesMut,
  ) -> Result<postgres_types::IsNull, Box<dyn std::error::Error + Send + Sync>> {
    if *ty == postgres_types::Type::UUID {
      let uuid: uuid::Uuid = self.0.parse()?;
      uuid.to_sql(ty, out)
    } else {
      self.0.to_sql(ty, out)
    }
  }

  fn accepts(ty: &postgres_types::Type) -> bool {
    *ty == postgres_types::Type::UUID || <String as postgres_types::ToSql>::accepts(ty)
  }

  postgres_types::to_sql_checked!();
}

/// `numeric` in text format, so a decimal is not sent as `float8`.
#[derive(Debug)]
struct PgNumeric(String);

impl postgres_types::ToSql for PgNumeric {
  fn to_sql(
    &self,
    _ty: &postgres_types::Type,
    out: &mut bytes::BytesMut,
  ) -> Result<postgres_types::IsNull, Box<dyn std::error::Error + Send + Sync>> {
    out.extend_from_slice(self.0.as_bytes());
    Ok(postgres_types::IsNull::No)
  }

  fn accepts(ty: &postgres_types::Type) -> bool {
    *ty == postgres_types::Type::NUMERIC
  }

  fn encode_format(&self, _ty: &postgres_types::Type) -> postgres_types::Format {
    postgres_types::Format::Text
  }

  postgres_types::to_sql_checked!();
}

/// Microseconds since 2000-01-01 UTC, the binary `timestamptz` representation.
#[derive(Debug)]
struct PgTimestamp(i64);

impl postgres_types::ToSql for PgTimestamp {
  fn to_sql(
    &self,
    _ty: &postgres_types::Type,
    out: &mut bytes::BytesMut,
  ) -> Result<postgres_types::IsNull, Box<dyn std::error::Error + Send + Sync>> {
    out.extend_from_slice(&self.0.to_be_bytes());
    Ok(postgres_types::IsNull::No)
  }

  fn accepts(ty: &postgres_types::Type) -> bool {
    *ty == postgres_types::Type::TIMESTAMPTZ
  }

  postgres_types::to_sql_checked!();
}

type PgParam = Box<dyn ToSql + Sync + Send>;

/// Convert a slice of [`Value`] into boxed [`ToSql`] trait objects for
/// `tokio_postgres::Client::query` / `execute`.
///
/// # Errors
///
/// [`DbCoreError::InvalidParameter`] when a tagged UUID, timestamp, JSON or
/// numeric value cannot be encoded.
pub fn to_pg_params(params: &[Value]) -> Result<Vec<PgParam>, DbCoreError> {
  params.iter().map(encode_one).collect()
}

fn encode_one(value: &Value) -> Result<PgParam, DbCoreError> {
  match value {
    Value::Null => Ok(Box::new(PgNull)),
    Value::Integer(n) => Ok(Box::new(*n)),
    Value::Real(f) => Ok(Box::new(*f)),
    Value::Text(s) => Ok(Box::new(FlexibleText(s.clone()))),
    Value::Blob(b) => Ok(Box::new(b.clone())),
    Value::Boolean(flag) => Ok(Box::new(*flag)),
    Value::TimestampEpoch(secs) => Ok(Box::new(PgTimestamp(epoch_secs_to_pg_micros(*secs)?))),
    Value::TimestampText(text) => Ok(Box::new(PgTimestamp(rfc3339_to_pg_micros(text)?))),
    Value::Json { text, storage } => encode_json(text, *storage),
    Value::Uuid(text) => encode_uuid(text),
    Value::Numeric(text) => encode_numeric(text),
  }
}

fn encode_json(text: &str, storage: JsonStorage) -> Result<PgParam, DbCoreError> {
  let parsed: serde_json::Value =
    serde_json::from_str(text).map_err(|error| invalid("json", &error.to_string()))?;
  match storage {
    JsonStorage::Json => Ok(Box::new(JsonValue(parsed))),
    JsonStorage::Jsonb => Ok(Box::new(JsonbValue(parsed))),
  }
}

fn encode_uuid(text: &str) -> Result<PgParam, DbCoreError> {
  let uuid = uuid::Uuid::parse_str(text).map_err(|error| invalid("uuid", &error.to_string()))?;
  Ok(Box::new(uuid))
}

fn encode_numeric(text: &str) -> Result<PgParam, DbCoreError> {
  if !is_decimal(text) {
    return Err(invalid("numeric", "not a decimal"));
  }
  Ok(Box::new(PgNumeric(text.to_owned())))
}

fn invalid(kind: &'static str, reason: &str) -> DbCoreError {
  DbCoreError::InvalidParameter {
    kind,
    reason: reason.to_owned(),
  }
}

/// `json` (not `jsonb`). A separate wrapper so `accepts` is one OID.
#[derive(Debug)]
struct JsonValue(serde_json::Value);

impl postgres_types::ToSql for JsonValue {
  fn to_sql(
    &self,
    ty: &postgres_types::Type,
    out: &mut bytes::BytesMut,
  ) -> Result<postgres_types::IsNull, Box<dyn std::error::Error + Send + Sync>> {
    self.0.to_sql(ty, out)
  }

  fn accepts(ty: &postgres_types::Type) -> bool {
    *ty == postgres_types::Type::JSON
  }

  postgres_types::to_sql_checked!();
}

/// `jsonb`. A separate wrapper so `accepts` is one OID.
#[derive(Debug)]
struct JsonbValue(serde_json::Value);

impl postgres_types::ToSql for JsonbValue {
  fn to_sql(
    &self,
    ty: &postgres_types::Type,
    out: &mut bytes::BytesMut,
  ) -> Result<postgres_types::IsNull, Box<dyn std::error::Error + Send + Sync>> {
    self.0.to_sql(ty, out)
  }

  fn accepts(ty: &postgres_types::Type) -> bool {
    *ty == postgres_types::Type::JSONB
  }

  postgres_types::to_sql_checked!();
}

fn is_decimal(text: &str) -> bool {
  if text.eq_ignore_ascii_case("nan") {
    return true;
  }
  let bytes = text.as_bytes();
  let mut at = 0_usize;
  if bytes.first() == Some(&b'+') || bytes.first() == Some(&b'-') {
    at += 1;
  }
  let mut digits = 0_usize;
  let mut dot = false;
  while let Some(byte) = bytes.get(at) {
    if byte.is_ascii_digit() {
      digits += 1;
      at += 1;
    } else if *byte == b'.' && !dot {
      dot = true;
      at += 1;
    } else {
      break;
    }
  }
  if digits == 0 {
    return false;
  }
  if bytes.get(at) == Some(&b'e') || bytes.get(at) == Some(&b'E') {
    at += 1;
    if bytes.get(at) == Some(&b'+') || bytes.get(at) == Some(&b'-') {
      at += 1;
    }
    let start = at;
    while bytes.get(at).is_some_and(u8::is_ascii_digit) {
      at += 1;
    }
    if at == start {
      return false;
    }
  }
  at == bytes.len()
}
