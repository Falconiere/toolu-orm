# Row mapping

`FromRow` turns a driver row into your struct. It carries the column list with
it, which is what makes `select_for::<T>()` safe:

```rust
pub trait FromRow: Sized {
  const REQUIRED_COLUMNS: &'static [&'static str];
  fn from_row(row: &Row) -> Result<Self, DbCoreError>;   // single-driver shape
}
```

`REQUIRED_COLUMNS` is the ordered column list `select_for::<T>()` passes to
`columns_raw`, so field order in the struct is the column order in the query and
positional reads line up.

## The shape depends on the driver set

The trait is feature-gated on `toolu-orm-core`:

| Drivers active | Methods |
|---|---|
| `libsql` only | `from_row(&libsql::Row)` |
| `rusqlite` only | `from_row(&rusqlite::Row<'_>)` |
| `postgres` only | `from_row(&tokio_postgres::Row)` |
| `postgres` + `libsql` | `from_pg_row`, `from_libsql_row` |
| `postgres` + `rusqlite` | `from_pg_row`, `from_rusqlite_row` |
| `libsql` + `rusqlite` | `from_libsql_row`, `from_rusqlite_row` |

An application runs one driver, so `from_row` is the usual shape.
`#[derive(FromRow)]` follows this table: it expands to whichever shape the
drivers on `toolu-orm-core` gave the trait, so it compiles on all seven
combinations.

## `#[derive(FromRow)]`

```rust
#[derive(FromRow)]
pub struct User {
  pub id: String,
  pub email: String,
  pub created_at: i64,
}
```

`REQUIRED_COLUMNS` is filled from the field names in declaration order, and each
field is read positionally at its own index with the field's own Rust type, so an
`Option<T>` field decodes SQL `NULL` as `None`.

Each driver gets a real decoder, spelled the way that driver reads a column:
`try_get::<usize, T>(i)` for Postgres, `get::<T>(i)` for libsql (which indexes
with `i32`), `get::<usize, T>(i)` for rusqlite. A field the driver cannot read
is a `DbCoreError::RowMapping` naming the column's index and name — never a
panic:

```text
column 3 (age): Invalid column index: 3
```

How the derive knows the shape is worth a note, because it cannot see
`toolu-orm-core`'s features: it expands inside *your* crate, where
`feature = "postgres"` means your feature. So it emits one decoder per driver
and hands all three to `toolu_orm_core::impl_derived_from_row!`, a macro whose
eight definitions are each `#[cfg]`-gated on `toolu-orm-core`'s own features.
A `macro_rules!` definition is compiled with its defining crate's features, so
the surviving definition is the one matching the shape that build compiled, and
the decoders for inactive drivers are dropped without ever being expanded. You
need no build script and no feature flags on the derive.

### Converting a field

`#[from_row(with = "f")]` routes the decoded value through `f`, which takes and
returns the field's own type (`FieldTy -> Result<FieldTy, E>`), so it normalizes
or rejects rather than converting between types. `f`'s error becomes the same
`RowMapping` message, naming the column.

```rust
fn normalize_email(raw: String) -> Result<String, DbCoreError> {
  if raw.contains('@') {
    Ok(raw.to_lowercase())
  } else {
    Err(DbCoreError::RowMapping(format!("{raw:?} is not an email")))
  }
}

#[derive(FromRow)]
pub struct Contact {
  pub id: String,
  #[from_row(with = "normalize_email")]
  pub email: String,
}
```

## Writing the impl by hand

Still supported, and the way out when a field type the active driver cannot
decode needs a real conversion — libsql's `FromValue` is a sealed trait, so
only its own set of types can decode there. Four lines per column, and no macro
between you and the driver:

```rust
use toolu_orm_core::{error::DbCoreError, row::FromRow};

impl FromRow for User {
  const REQUIRED_COLUMNS: &'static [&'static str] = &["id", "email", "created_at"];

  fn from_row(row: &libsql::Row) -> Result<Self, DbCoreError> {
    let col = |i: i32, e: libsql::Error| DbCoreError::RowMapping(format!("col {i}: {e}"));
    Ok(Self {
      id: row.get(0).map_err(|e| col(0, e))?,
      email: row.get(1).map_err(|e| col(1, e))?,
      created_at: row.get(2).map_err(|e| col(2, e))?,
    })
  }
}
```

For Postgres the row is a `tokio_postgres::Row` and columns can be read by name
(`row.try_get("email")`). Nullable columns map to `Option<T>`; the index passed
to `row.get` must match the position of the column in `REQUIRED_COLUMNS`.

`toolu_orm_core::impl_from_row_for!` is available when you need to write one impl
that covers several driver shapes at once — that is what the multi-driver test
suites use.

## Where it is used

Anything that returns rows is generic over `FromRow`:

```rust
// `exec` is the driver connection (Executor); `conn` is the DbConnection wrapper.
let exec = conn.inner_conn();

let users: Vec<User> = UsersTable::select_for::<User>().fetch_all(exec).await?;
let one: User        = UsersTable::select_for::<User>().filter(users::id.eq("u_1")).fetch_one(exec).await?;
let rows: Vec<User>  = conn.query_map::<User>("SELECT id, email, created_at FROM users", vec![]).await?;
```

The two are different surfaces — see [Connections](../drivers/index.md).
