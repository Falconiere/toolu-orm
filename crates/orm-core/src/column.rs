//! Column types, definitions, and dialect-aware DDL generation.

use serde::{Deserialize, Serialize};

use crate::dialect::Dialect;

/// The element type of a `vec0` vector column.
///
/// Fixed at creation together with the dimension: `sqlite-vec` has no way to
/// widen or re-type a vector in place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub enum VectorElement {
  Float,
  Int8,
  Bit,
}

impl VectorElement {
  /// The name `vec0` reads in a column definition, as in `float[768]`.
  #[must_use]
  pub fn as_vec0_sql(self) -> &'static str {
    match self {
      Self::Float => "float",
      Self::Int8 => "int8",
      Self::Bit => "bit",
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ColumnType {
  // Core SQLite
  Text,
  Integer,
  Real,
  Blob,
  // Turso STRICT types
  Uuid,
  Boolean,
  Timestamp,
  Date,
  Time,
  Json,
  BigInt,
  SmallInt,
  Varchar(u32),
  // Postgres-oriented types (usable on both dialects)
  Serial,
  BigSerial,
  Jsonb,
  Numeric,
  Char(u32),
  Array(Box<ColumnType>),
  /// A `vec0` vector, e.g. `FLOAT[1024]`. Both parts are the column's
  /// identity, so a change to either is a new column to the diff. Outside a
  /// `vec0` table the value is just its bytes: `BLOB`, or `BYTEA` on Postgres.
  Vector {
    element: VectorElement,
    dim: u32,
  },
}

impl ColumnType {
  /// SQL type name for Turso STRICT tables (extension types).
  pub fn as_sql(&self) -> String {
    match self {
      Self::Text | Self::Jsonb | Self::Char(_) | Self::Array(_) => "TEXT".to_owned(),
      Self::Integer | Self::Serial | Self::BigSerial => "INTEGER".to_owned(),
      Self::Real | Self::Numeric => "REAL".to_owned(),
      Self::Blob | Self::Vector { .. } => "BLOB".to_owned(),
      Self::Uuid => "uuid".to_owned(),
      Self::Boolean => "boolean".to_owned(),
      Self::Timestamp => "timestamp".to_owned(),
      Self::Date => "date".to_owned(),
      Self::Time => "time".to_owned(),
      Self::Json => "json".to_owned(),
      Self::BigInt => "bigint".to_owned(),
      Self::SmallInt => "smallint".to_owned(),
      Self::Varchar(n) => format!("varchar({n})"),
    }
  }

  /// SQL type for non-STRICT tables (standard SQLite compatibility).
  pub fn as_compat_sql(&self) -> String {
    match self {
      Self::Text
      | Self::Uuid
      | Self::Date
      | Self::Time
      | Self::Json
      | Self::Jsonb
      | Self::Varchar(_)
      | Self::Char(_)
      | Self::Array(_) => "TEXT".to_owned(),
      Self::Integer
      | Self::BigInt
      | Self::SmallInt
      | Self::Timestamp
      | Self::Boolean
      | Self::Serial
      | Self::BigSerial => "INTEGER".to_owned(),
      Self::Real | Self::Numeric => "REAL".to_owned(),
      Self::Blob | Self::Vector { .. } => "BLOB".to_owned(),
    }
  }

  /// DDL type name for the given dialect. Used by the SQL generator (`sql.rs`)
  /// to produce CREATE TABLE / ALTER TABLE statements.
  ///
  /// This is separate from [`Self::as_sql()`] which returns Turso STRICT type names
  /// (e.g. `"uuid"`, `"boolean"`) for backward compatibility.
  pub fn as_ddl_sql(&self, dialect: Dialect) -> String {
    match dialect {
      Dialect::Sqlite => match self {
        Self::Text
        | Self::Uuid
        | Self::Date
        | Self::Time
        | Self::Json
        | Self::Jsonb
        | Self::Varchar(_)
        | Self::Char(_)
        | Self::Array(_) => "TEXT".to_owned(),
        Self::Integer
        | Self::Boolean
        | Self::Timestamp
        | Self::BigInt
        | Self::SmallInt
        | Self::Serial
        | Self::BigSerial => "INTEGER".to_owned(),
        Self::Real | Self::Numeric => "REAL".to_owned(),
        Self::Blob | Self::Vector { .. } => "BLOB".to_owned(),
      },
      Dialect::Postgres => match self {
        Self::Text => "TEXT".to_owned(),
        Self::Integer => "INTEGER".to_owned(),
        Self::Real => "DOUBLE PRECISION".to_owned(),
        Self::Blob | Self::Vector { .. } => "BYTEA".to_owned(),
        Self::Uuid => "UUID".to_owned(),
        Self::Boolean => "BOOLEAN".to_owned(),
        Self::Timestamp => "TIMESTAMPTZ".to_owned(),
        Self::Date => "DATE".to_owned(),
        Self::Time => "TIME".to_owned(),
        Self::Json => "JSON".to_owned(),
        Self::Jsonb => "JSONB".to_owned(),
        Self::BigInt => "BIGINT".to_owned(),
        Self::SmallInt => "SMALLINT".to_owned(),
        Self::Serial => "SERIAL".to_owned(),
        Self::BigSerial => "BIGSERIAL".to_owned(),
        Self::Numeric => "NUMERIC".to_owned(),
        Self::Varchar(n) => format!("VARCHAR({n})"),
        Self::Char(n) => format!("CHAR({n})"),
        Self::Array(inner) => format!("{}[]", inner.as_ddl_sql(Dialect::Postgres)),
      },
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ForeignKeyAction {
  Cascade,
  SetNull,
  SetDefault,
  Restrict,
  NoAction,
}

impl ForeignKeyAction {
  pub fn as_sql(self) -> &'static str {
    match self {
      Self::Cascade => "CASCADE",
      Self::SetNull => "SET NULL",
      Self::SetDefault => "SET DEFAULT",
      Self::Restrict => "RESTRICT",
      Self::NoAction => "NO ACTION",
    }
  }
}

/// Trait for Rust enums that map to TEXT columns with CHECK constraints.
pub trait EnumSchema {
  fn variants() -> &'static [&'static str];
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColumnDef {
  pub name: String,
  pub column_type: ColumnType,
  pub primary_key: bool,
  pub not_null: bool,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub default: Option<String>,
  pub unique: bool,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub references: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub on_delete: Option<ForeignKeyAction>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub on_update: Option<ForeignKeyAction>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub check: Option<String>,
  /// FTS5 `UNINDEXED`: the column is stored but not searchable. Ignored on
  /// ordinary tables, and defaulted so older snapshots still deserialize.
  #[serde(default, skip_serializing_if = "std::ops::Not::not")]
  pub unindexed: bool,
}

// Marker types for schema definition — used by the #[table] proc macro.
pub struct Text;
pub struct Integer;
pub struct Real;
pub struct Blob;
pub struct Uuid;
pub struct Boolean;
pub struct Timestamp;
pub struct Date;
pub struct Time;
pub struct Json;
pub struct BigInt;
pub struct SmallInt;
pub struct Varchar<const N: u32>;
pub struct Serial;
pub struct BigSerial;
pub struct Jsonb;
pub struct Numeric;
pub struct Char<const N: u32>;
/// `#[vec0_table]`'s vector marker; the dimension comes from `#[column(dim = N)]`.
pub struct Vector;
