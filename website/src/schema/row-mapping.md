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

An application runs one driver, so `from_row` is the usual shape — and the one
`#[derive(FromRow)]` cannot produce.

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

> **The derive only compiles on the postgres + libsql shape.** It always emits
> `from_pg_row` (real decoding) plus a `from_libsql_row` error stub, so on a
> single-driver lane — libsql-only, rusqlite-only, or postgres-only — the trait
> asks for `from_row` and the derive does not provide it: the build fails with a
> missing-method error. Only the postgres lane, where both features are unified
> on `toolu-orm-core`, can derive it. Everywhere else, write the impl by hand as
> shown below. Its libsql method is a stub even there, so a derived type decodes
> Postgres rows and returns `DbCoreError::RowMapping` on a libsql row.

## Writing the impl by hand

Four lines per column, and no macro between you and the driver:

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
