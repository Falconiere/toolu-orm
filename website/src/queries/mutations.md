# Insert, update, delete

The three write builders mirror `SelectBuilder`: build, render with `to_sql_for`,
execute with `.execute(conn)` — which returns the number of affected rows.

`conn` here is the **driver** connection (`&libsql::Connection`,
`&rusqlite::Connection`, `&tokio_postgres::Client`, or a transaction), not the
`DbConnection` wrapper; see [Connections](../drivers/index.md). On rusqlite the
call is synchronous, without `.await`.

## Insert

```rust
use toolu_orm_query::insert::InsertBuilder;

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
| `.or_ignore()` | Skip the row on conflict. |
| `.or_replace()` | Overwrite the conflicting row. |
| `.conflict_columns(&["run_id"])` | The conflict target for Postgres. |

### Upsert per dialect

The same builder renders the idiomatic form for each database:

```rust
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
column when none is given. Be explicit when the table has more than one unique
constraint.

## Update

```rust
use toolu_orm_query::update::UpdateBuilder;

UpdateBuilder::new("users")
  .set(&users::email, "new@example.com")
  .set_expr(&users::updated_at, "unixepoch()")
  .filter(users::id.eq("user-1"))
  .execute(&conn)
  .await?;
// UPDATE "users" SET "email" = ?1, "updated_at" = unixepoch() WHERE "users"."id" = ?2
```

`set` binds a parameter; `set_expr` writes raw SQL on the right-hand side, which
is how you do `counter = counter + 1` or a database-side timestamp. Assignments
render in insertion order and the `WHERE` parameters are numbered after them.

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

## From the table type

The generated factories save the table name:

```rust
UsersTable::insert().set(&users::id, "u_1").execute(&conn).await?;
UsersTable::update().set(&users::email, "x@y.z").filter(users::id.eq("u_1")).execute(&conn).await?;
UsersTable::delete().filter(users::id.eq("u_1")).execute(&conn).await?;
```

All three take the same executor as select — a connection, or a transaction. See
[Transactions](transactions.md).
