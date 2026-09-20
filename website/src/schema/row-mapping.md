# Row mapping

`FromRow` turns a driver row into your struct. It carries the column list with
it, so `select_for::<T>()` can initialize a matching projection:

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
| none | no decoder method; only `REQUIRED_COLUMNS` |
| `libsql` only | `from_row(&libsql::Row)` |
| `rusqlite` only | `from_row(&rusqlite::Row<'_>)` |
| `postgres` only | `from_row(&tokio_postgres::Row)` |
| `postgres` + `libsql` | `from_pg_row`, `from_libsql_row` |
| `postgres` + `rusqlite` | `from_pg_row`, `from_rusqlite_row` |
| `libsql` + `rusqlite` | `from_libsql_row`, `from_rusqlite_row` |
| all three | `from_pg_row`, `from_libsql_row`, `from_rusqlite_row` |

An application runs one driver, so `from_row` is the usual shape.
`#[derive(FromRow)]` follows this table: it expands to whichever shape the
drivers on `toolu-orm-core` gave the trait, including the no-driver shape. Field
types still need to support decoding on every enabled driver.

## `#[derive(FromRow)]`

```rust
use toolu_orm_macros::FromRow;

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
with `i32`), `get::<usize, T>(i)` for rusqlite. A runtime decoding failure
produces a `DbCoreError::RowMapping` naming
the column's index and name:

```text
column 3 (age): Invalid column index: 3
```

Unsupported Rust field types fail to compile. Driver behavior for a missing
positional column also differs: libsql treats an out-of-range column as SQL
`NULL`, so a missing trailing `Option<T>` can become `None`; rusqlite reports
an invalid column index. Use `select_for::<T>()` or an explicit ordered column
list when writing raw SQL.

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
`RowMapping` message, naming the column. `f` must be a simple function name in
scope; a path such as `module::normalize_email` is not accepted.

```rust
use toolu_orm_core::error::DbCoreError;

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
use toolu_orm_core::{error::DbCoreError, libsql, row::FromRow};

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

This hand-written example replaces the derive above and assumes libsql is the
only enabled core driver. When writing a manual impl for multiple drivers, use
the corresponding methods in the table above. `impl_from_row_for!` can emit
single/dual/triple method shapes, but its caller-supplied `#[cfg]` is evaluated
in the consuming crate. It does not discover Cargo's unified core features.
Callers that only decode rows can use `row::from_libsql_row`,
`row::from_rusqlite_row` or `row::from_postgres_row` to avoid naming the trait's
feature-dependent method.

## Where it is used

Ordinary typed query results use `FromRow` (`Relational` has its own mapping):

```rust
use toolu_orm_connection::DbConnection;
use toolu_orm_core::query_column::CommonOps;

// `exec` is the driver connection (Executor); `conn` is the DbConnection wrapper.
let exec = conn.inner_conn();

let users: Vec<User> = UsersTable::select_for::<User>().fetch_all(exec).await?;
let one: User        = UsersTable::select_for::<User>().filter(users::id.eq("u_1")).fetch_one(exec).await?;
let rows: Vec<User>  = conn.query_map::<User>("SELECT id, email, created_at FROM users", vec![]).await?;
```

The two are different surfaces — see [Connections](../drivers/index.md).
