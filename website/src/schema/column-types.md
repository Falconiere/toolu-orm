# Column types

Field types on a `#[table]` struct are **marker types**, not storage types. They
select a `ColumnType`, which decides the DDL for each dialect.

```rust
use toolu_orm_core::column::{BigInt, BigSerial, Blob, Boolean, Char, Date, Integer, Json, Jsonb,
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
| `Timestamp` | `Timestamp` | `INTEGER` on plain SQLite; `TIMESTAMPTZ` on Postgres. |
| `Date` / `Time` | `Date` / `Time` | `TEXT` on plain SQLite; `DATE` / `TIME` on Postgres. |
| `Json` / `Jsonb` | `Json` / `Jsonb` | `Jsonb` falls back to `TEXT` on SQLite. |
| `BigInt` / `SmallInt` | `BigInt` / `SmallInt` | |
| `Varchar<255>` | `Varchar(255)` | Length is a literal const generic; plain SQLite renders `TEXT`. |
| `Char<2>` | `Char(2)` | `TEXT` on SQLite. |
| `Serial` / `BigSerial` | `Serial` / `BigSerial` | `INTEGER` on SQLite. |
| `Numeric` | `Numeric` | `REAL` on SQLite. |

```rust
use toolu_orm_core::column::{Char, Text, Uuid, Varchar};
use toolu_orm_macros::table;

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

For SQLite, `strict = true` selects the Turso extension names from `ColumnType::as_sql()`;
otherwise the generator uses ordinary SQLite storage types. Stock SQLite STRICT
tables do not accept extension names such as `uuid`, `boolean`, `timestamp` or
`varchar(64)`. Use supported core types (`Text`, `Integer`, `Real`, `Blob`) when
targeting stock SQLite STRICT tables. Postgres always uses its own DDL names and
ignores `strict`.

`ColumnType::Array(Box<ColumnType>)` is available for programmatic schema
definitions: it renders a native array type on Postgres and `TEXT` on SQLite.
There is no `Array` marker or `#[table]` array syntax.

### Virtual-table marker

`Vector` is a separate marker for `#[vec0_table]`, not ordinary `#[table]`
columns. It requires `#[column(dim = N)]` to supply the dimension;
see [Virtual tables](tables.md#virtual-tables).

## `Value` — what actually gets bound

Parameters are bound as `toolu_orm_core::value::Value`. Untyped values are
`Null`, `Integer`, `Real`, `Text` and `Blob`. `Into<Value>` is implemented for
`&str`, `String`, `i32`, `i64`, `f64`, `bool` (as `0` / `1`), `Vec<u8>`, and
`Option<T>` where `T: Into<Value>` (`None` becomes `Value::Null`). So
`.set(&users::email, "a@b.c")` and `.filter(users::attempts.gt(3i32))` take
plain Rust values. `From<bool>` stays an integer, and an integer column given
`true` still binds `1`.

A bind made through `Column<Boolean>`, `Column<Timestamp>`, `Column<Json>`,
`Column<Jsonb>`, `Column<Uuid>` or `Column<Numeric>` retags that value before
the driver sees it. The same `Value` is stored for both engines; SQLite maps
the tag back to an integer or text when it binds.

| Marker | What you pass | Postgres parameter | SQLite parameter |
|---|---|---|---|
| `Boolean` | `bool`, or `0` / `1` | `boolean` | integer `0` / `1` |
| `Timestamp` | unix-epoch `i64`, or RFC3339 text | `timestamptz` | the integer, or the text |
| `Json` / `Jsonb` | JSON text | `json` / `jsonb` | the text |
| `Uuid` | UUID text | `uuid` | the text |
| `Numeric` | decimal text, an integer, or a finite float | `numeric` | text |

A UUID, timestamp, JSON value or numeric that cannot be encoded is refused
before the driver is asked to send it. `LIKE` is not retagged: its argument is
a pattern, not a uuid or a timestamp. Untyped `Expr::raw` and an integer column
still send `int8`.

`TextOps` is implemented for `Text`, `Uuid`, `Date`, `Time` and `Varchar<N>`;
`NumericOps` for `Integer`, `Real`, `BigInt`, `SmallInt`, `Timestamp`, `Date`
and `Time`. Other markers still have `CommonOps`; their storage type does not
automatically add those specialized traits.

For sqlite-vec, `Value::vector(&[f32])` encodes little-endian float bytes as a
blob; `Value::vector_with_dim(&[f32], dim)` also checks the dimension. These
helpers do not encode Postgres pgvector values.

## Nullability

`#[column(not_null)]` sets an explicit schema constraint; an `Option<Marker>`
field is not the way to declare nullability. Primary-key rules also affect
nullability and differ by engine, so declare `not_null` when you need that
guarantee on every backend. On the read side, map nullable values to `Option<T>`
in your row struct (see [Row mapping](row-mapping.md)).
