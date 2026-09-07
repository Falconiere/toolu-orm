# Select

`SelectBuilder` renders a `SELECT` for either dialect and, when a single driver
is active, executes it.

```rust
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::query_column::{CommonOps, NumericOps};
use toolu_orm_query::select::SelectBuilder;

let (sql, params) = SelectBuilder::new("users")
  .columns_raw(&["id", "email"])
  .filter(users::org_id.eq("org123"))
  .filter(users::created_at.gt(0i32))
  .join("pipelines", users::id.equals(&pipelines::user_id))
  .order_by(users::created_at.desc())
  .limit(10)
  .offset(0)
  .to_sql_for(Dialect::Sqlite);
```

```sql
SELECT "id", "email" FROM "users"
  INNER JOIN "pipelines" ON "users"."id" = "pipelines"."user_id"
  WHERE "users"."org_id" = ?1 AND "users"."created_at" > ?2
  ORDER BY "users"."created_at" DESC LIMIT ?3 OFFSET ?4
```

Column references are table-qualified and quoted; `limit` and `offset` are bound
as parameters, not interpolated.

## Building

| Method | Effect |
|---|---|
| `SelectBuilder::new(table)` | Start a select on `table`. `Table::select()` does the same. |
| `SelectBuilder::raw()` | Start with no `FROM` table — for expression-only selects. |
| `.columns_raw(&["id", "email"])` | Explicit column list. Without one, the select is `SELECT *`. |
| `.columns_typed(&[&users::id, &users::email])` | Same, from typed column references. |
| `.column_expr("COUNT(*)", "n")` | Add a raw expression with an alias. |
| `.filter(expr)` | Add a predicate. Repeated calls are `AND`-ed. See [Filters](filters.md). |
| `.join(table, on)` / `.left_join(table, on)` | `INNER` / `LEFT JOIN`, with `on` built by `Column::equals`. |
| `.order_by(users::created_at.desc())` | `ORDER BY`. `asc()` and `desc()` come from `Column`. |
| `.limit(n)` / `.offset(n)` | Paging, bound as parameters. |
| `.table_name()` | The table this builder targets. |

## Rendering

Every builder renders to `(String, Vec<Value>)`:

| Method | Returns |
|---|---|
| `.to_sql_for(dialect)` | SQL for that dialect — `?N` for `Dialect::Sqlite`, `$N` for `Dialect::Postgres`. |
| `.to_sql()` | Same, for `Dialect::CURRENT` (the dialect implied by the active features). |
| `.to_count_sql_for(dialect)` / `.to_count_sql()` | The same query wrapped in `COUNT(*)`. |
| `.to_exists_sql_for(dialect)` / `.to_exists_sql()` | The same query as an existence check. |

Rendering never touches the database, which is why the builders compile with any
feature combination and are straightforward to unit test.

## Executing

With exactly one driver feature active, the select gains its fetch methods. They
take the **driver** connection — `&libsql::Connection`, `&rusqlite::Connection`,
`&tokio_postgres::Client` or a transaction — which the connection wrappers expose
(`conn.inner_conn()` on libsql). See [Connections](../drivers/index.md).

```rust
let all: Vec<User> = UsersTable::select_for::<User>().fetch_all(&conn).await?;

let one: User = UsersTable::select_for::<User>()
  .filter(users::id.eq("u_1"))
  .fetch_one(&conn)          // QueryError::NotFound when there is no row
  .await?;

let maybe: Option<User> = UsersTable::select_for::<User>()
  .filter(users::id.eq("nope"))
  .fetch_optional(&conn)     // None when there is no row
  .await?;

let n: i64      = UsersTable::select().count(&conn).await?;
let any: bool   = UsersTable::select().filter(users::org_id.eq("org123")).exists(&conn).await?;
```

`fetch_all`, `fetch_one` and `fetch_optional` are generic over
[`FromRow`](../schema/row-mapping.md); `count` and `exists` are not, they render
the count/exists form of the query.

`select_for::<T>()` sets the column list from `T::REQUIRED_COLUMNS`, so the
projection always matches the struct being decoded. Use `select()` plus
`columns_raw` when the shape is not a `FromRow` struct.

On the rusqlite driver these methods are synchronous — same names, no `.await`.

> Multiple driver features on `toolu-orm-query` disable the executor: the
> builders and `to_sql_for` still work, `fetch_*` is not compiled. See
> [Installation](../getting-started/installation.md).
