# Column types

Field types on a `#[table]` struct are **marker types**, not storage types. They
select a `ColumnType`, which decides the DDL for each dialect.

```rust
use toolu_orm_core::column::{BigInt, Blob, Boolean, Char, Date, Integer, Json, Jsonb,
                            Numeric, Real, Serial, SmallInt, Text, Time, Timestamp,
                            Uuid, Varchar};
```

| Marker | `ColumnType` | Notes |
|---|---|---|
| `Text` | `Text` | |
| `Integer` | `Integer` | |
| `Real` | `Real` | |
| `Blob` | `Blob` | |
| `Uuid` | `Uuid` | `uuid` on STRICT/Postgres, `TEXT` on plain SQLite. |
| `Boolean` | `Boolean` | `INTEGER` on plain SQLite. |
| `Timestamp` | `Timestamp` | |
| `Date` / `Time` | `Date` / `Time` | |
| `Json` / `Jsonb` | `Json` / `Jsonb` | `Jsonb` falls back to `TEXT` on SQLite. |
| `BigInt` / `SmallInt` | `BigInt` / `SmallInt` | |
| `Varchar<255>` | `Varchar(255)` | Length is a const generic. |
| `Char<2>` | `Char(2)` | `TEXT` on SQLite. |
| `Serial` / `BigSerial` | `Serial` / `BigSerial` | `INTEGER` on SQLite. |
| `Numeric` | `Numeric` | `REAL` on SQLite. |

```rust
#[table(name = "accounts")]
pub struct Accounts {
  #[column(primary_key)]
  pub id: Uuid,
  #[column(not_null)]
  pub country: Char<2>,
  #[column(not_null)]
  pub slug: Varchar<64>,
  #[column(not_null, column_type = "Jsonb")]
  pub settings: Text,
}
```

Three renderings exist for each type: the STRICT SQL name, the plain-SQLite
compatibility name, and the Postgres name. `strict = true` on the table picks the
first; the dialect passed to migration generation picks between the others.

## `Value` — what actually gets bound

Parameters are bound as `toolu_orm_core::value::Value`, a five-variant enum:

```rust
pub enum Value {
  Null,
  Integer(i64),
  Real(f64),
  Text(String),
  Blob(Vec<u8>),
}
```

`Into<Value>` is implemented for `&str`, `String`, `i32`, `i64`, `f64`, `bool`
(as `0` / `1`), `Vec<u8>`, and `Option<T>` where `T: Into<Value>` (`None` becomes
`Value::Null`). So `.set(&users::email, "a@b.c")` and
`.filter(users::attempts.gt(3i32))` take plain Rust values.

The column marker only constrains which operators are available — `like` on text,
the numeric comparisons on numbers — it does not convert values. Anything that
converts into a `Value` can be bound to any column.

## Nullability

Nullability is a schema property, not a Rust type property: a field without
`#[column(not_null)]` is nullable. On the read side, map it to `Option<T>` in your
row struct (see [Row mapping](row-mapping.md)).
