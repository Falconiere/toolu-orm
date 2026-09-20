# Insert, update, delete

The three write builders mirror `SelectBuilder`: build, render with `to_sql_for`,
execute with `.execute(conn)` — which returns the number of affected rows.

Execution requires exactly one query driver feature. `conn` implements that
driver's `Executor`: `libsql::Connection`, `rusqlite::Connection`,
`tokio_postgres::Client`, or a supported query transaction wrapper.
`toolu_orm_connection::RusqliteConnection` also works directly. On rusqlite the
call is synchronous, without `.await`; libsql and Postgres are async.
See [Connections](../drivers/index.md).

## Insert

```rust
use toolu_orm_query::insert::InsertBuilder;
use toolu_orm_core::query_column::CommonOps;

let affected = InsertBuilder::new("users")
  .set(&users::id, "u_1")
  .set(&users::email, "ada@example.com")
  .set_null(&users::deleted_at)
  .execute(&conn)
  .await?;
```

| Method | Effect |
|---|---|
| `.set(&col, value)` | Bind a column. Any `Into<Value>` works. |
| `.set_null(&col)` | Bind SQL `NULL` explicitly. |
| `.set_scalar(&col, scalar)` | Assign a computed SQL expression, including its bound values. |
| `.on_conflict(clause)` | Explicit `ON CONFLICT` target and action on either dialect. |
| `.returning(&col)` | Append a returned column; read it with `fetch_*`. |
| `.or_ignore()` | SQLite `INSERT OR IGNORE`; Postgres `ON CONFLICT DO NOTHING`. |
| `.or_replace()` | SQLite deletes and reinserts; Postgres updates non-target inserted columns. |
| `.conflict_columns(&["run_id"])` | The target for the Postgres `or_replace` shorthand only. |

### Updating an existing row on conflict

`OnConflict` updates in place, preserving columns the update does not name.
Its target must match a unique constraint or index:

```rust
use toolu_orm_query::insert::OnConflict;

InsertBuilder::new("users")
  .set(&users::id, "u_1")
  .set(&users::email, "ada@example.com")
  .on_conflict(OnConflict::column(&users::id).set_excluded(&users::email))
  .execute(&conn)
  .await?;
```

Use `.and_column(&col)` for a composite target. The default action is
`DO NOTHING`; `set`, `set_scalar` and `set_excluded` change it to `DO UPDATE`.
Inside an assignment, `Scalar::col(&col)` reads the existing row and
`Scalar::excluded(&col)` reads the proposed insert. The last conflict policy
set on a builder wins; `on_conflict` has its own target and ignores
`conflict_columns`.

### Shorthands per dialect

The same builder renders the idiomatic form for each database:

```rust
use toolu_orm_core::dialect::Dialect;

InsertBuilder::new("seeds").or_ignore().set(&seeds::id, "seed-1").to_sql_for(Dialect::Sqlite);
// INSERT OR IGNORE INTO "seeds" ("id") VALUES (?1)

InsertBuilder::new("seeds").or_ignore().set(&seeds::id, "seed-1").to_sql_for(Dialect::Postgres);
// INSERT INTO "seeds" ("id") VALUES ($1) ON CONFLICT DO NOTHING

InsertBuilder::new("run_status")
  .or_replace()
  .conflict_columns(&["run_id"])
  .set(&run_status::run_id, "run-1")
  .set(&run_status::status, "running")
  .to_sql_for(Dialect::Postgres);
// INSERT INTO "run_status" ("run_id", "status") VALUES ($1, $2)
//   ON CONFLICT ("run_id") DO UPDATE SET "status" = EXCLUDED."status"
```

On SQLite `or_replace` is `INSERT OR REPLACE`, which needs no conflict target;
on Postgres the target comes from `conflict_columns`, defaulting to the first
inserted column when none is given. SQLite's replacement deletes the existing
row, resets omitted columns to their defaults, and can delete referencing rows
through `ON DELETE CASCADE`. Use `on_conflict` to preserve the stored row.

### Returning values

```rust
// UserId implements FromRow for the returned id column.
let inserted: Option<UserId> = InsertBuilder::new("users")
  .set(&users::id, "u_1")
  .set(&users::email, "ada@example.com")
  .on_conflict(OnConflict::column(&users::id).do_nothing())
  .returning(&users::id)
  .fetch_optional(&conn)
  .await?;
```

`fetch_optional` returns `None` when `DO NOTHING` skips the row; `fetch_one`
returns `QueryError::NotFound` in that case. Use `fetch_all` for multiple returned
rows. A `RETURNING` statement should use a fetch method: rusqlite rejects
`execute` for row-producing SQL, while the async executors discard the returned
rows. These methods are currently on `InsertBuilder`, not update or delete.
See the [upsert scenarios](https://github.com/Falconiere/toolu-orm/blob/main/docs/scenarios/upsert.md).

### Inserting from a query

Copy matching rows within the database using `.select(target_columns, source)`
or `.select_raw(target_column_names, source)`:

```rust
use toolu_orm_query::select::SelectBuilder;

let source = SelectBuilder::new("users")
  .columns_raw(&["id", "email"])
  .filter(users::status.eq("active"));
let copy = InsertBuilder::new("user_archive")
  .select_raw(&["id", "email"], source);
```

The target list must match the source projection's number and order. Once a
select source is set, all `set*` values are ignored, including later calls.
Conflict policies and `returning` still work. For a database-qualified target,
use `InsertBuilder::into_table(TableRef::new("users").in_database("main"))`;
`"main.users"` passed to `new` is one identifier, not a qualifier.
See [INSERT … SELECT](https://github.com/Falconiere/toolu-orm/blob/main/docs/scenarios/insert-select.md).

## Update

```rust
use toolu_orm_query::update::UpdateBuilder;

UpdateBuilder::new("users")
  .set(&users::email, "new@example.com")
  .set_expr(&users::updated_at, Dialect::CURRENT.now_epoch())
  .filter(users::id.eq("user-1"))
  .execute(&conn)
  .await?;
// UPDATE "users" SET "email" = ?1, "updated_at" = unixepoch() WHERE "users"."id" = ?2
```

`set` binds a parameter, including `Value::Null` for SQL NULL; `set_expr` writes
trusted raw SQL on the right-hand side. `set_scalar` accepts expressions with
parameters, such as `Scalar::col(&users::attempts) + Scalar::bind(1_i64)`.
Assignments render in insertion order and the `WHERE` parameters follow them.
`Dialect::CURRENT.now_epoch()` chooses the current driver's epoch expression;
arbitrary raw SQL is not translated between dialects. The SQL comment above
shows the SQLite form.

An `UpdateBuilder` with no filter updates every row. There is no guard against
that — add the `.filter()`.

## Delete

```rust
use toolu_orm_query::delete::DeleteBuilder;

DeleteBuilder::new("users")
  .filter(users::id.eq("user-1"))
  .execute(&conn)
  .await?;
// DELETE FROM "users" WHERE "users"."id" = ?1
```

Like an update, a delete with no filter affects every row. Both accept subquery
predicates, such as `Scalar::col(&users::id).in_subquery(select)`, for set-based
writes without loading the matching IDs into Rust.

## From the table type

The generated factories save the table name:

```rust
UsersTable::insert().set(&users::id, "u_1").execute(&conn).await?;
UsersTable::update().set(&users::email, "x@y.z").filter(users::id.eq("u_1")).execute(&conn).await?;
UsersTable::delete().filter(users::id.eq("u_1")).execute(&conn).await?;
```

All three take the same executor as select — a connection, or a transaction. See
[Transactions](transactions.md).
