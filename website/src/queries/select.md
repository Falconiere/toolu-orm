# Select

`SelectBuilder` renders a `SELECT` for either dialect and, when a single driver
is active, executes it.

```rust
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::query_column::{CommonOps, NumericOps};
use toolu_orm_query::select::SelectBuilder;

let (sql, params) = SelectBuilder::new("users")
  .columns_qualified(&[&users::id, &users::email])
  .filter(users::org_id.eq("org123"))
  .filter(users::created_at.gt(0i32))
  .join("pipelines", users::id.equals(&pipelines::user_id))
  .order_by(users::created_at.desc())
  .limit(10)
  .offset(0)
  .to_sql_for(Dialect::Sqlite);
```

```sql
SELECT "users"."id", "users"."email" FROM "users"
  INNER JOIN "pipelines" ON "users"."id" = "pipelines"."user_id"
  WHERE "users"."org_id" = ?1 AND "users"."created_at" > ?2
  ORDER BY "users"."created_at" DESC LIMIT ?3 OFFSET ?4
```

Column references are table-qualified and quoted; `limit` and `offset` are bound
as parameters, not interpolated.

An offset without a limit is supported: SQLite adds the literal `LIMIT -1`
before the bound offset; Postgres renders `OFFSET` alone.

## Building

| Method | Effect |
|---|---|
| `SelectBuilder::new(table)` | Start a select on `table`. `Table::select()` does the same. |
| `SelectBuilder::from_table(TableRef::aliased(t, a))` | Start a select on `t` under the alias `a` — `FROM "t" AS "a"`. `&str` and `&TableRef` also convert. |
| `SelectBuilder::raw()` | Start with no `FROM` table — for expression-only selects. |
| `.columns_raw(&["id", "email"])` | Set the column list to these quoted, bare names. These are names, not SQL expressions. |
| `.columns_typed(&[&users::id, &users::email])` | Same, from typed column references, under bare names. |
| `.columns_qualified(&[&users::id, &u.column(&users::id)])` | Same, qualified by table or alias — what a joined query needs. |
| `.column_as(&f.column(&feedback::id), "f_id")` | Add one qualified column under an output alias. |
| `.column_expr("COUNT(*)", "n")` | Add a raw expression with an alias. |
| `.column_scalar(scalar, "n")` | Add a computed expression that may bind values. See [Filters and expressions](filters.md#scalar-expressions). |
| `.filter(expr)` | Add a predicate. Repeated calls are `AND`-ed. See [Filters](filters.md). |
| `.join(table, on)` / `.left_join(table, on)` | `INNER` / `LEFT JOIN`. `table` is a `&str` or a `TableRef`; `on` is a `JoinCondition` or an `Expr`, so `a.equals(&b).and(live.eq(1))` works. |
| `.order_by(users::created_at.desc())` | `ORDER BY`. `asc()` and `desc()` come from `Column`. |
| `.limit(n)` / `.offset(n)` | Paging, bound as parameters. |
| `.table_name()` | The table this builder targets, never its alias. |
| `.table_ref()` | The same table, alias included. |

Choose a projection before fetching or rendering a row query: `new()` and the
generated `Table::select()` start with an empty select list, not `*`.
`Table::select_for::<Row>()` supplies the row type's required columns, and
`.columns_raw(users::ALL_COLUMNS)` selects every declared column.
The `columns_*` methods replace the plain column list; aliased expressions
are appended after it in their own call order.

For a self-join, qualify columns through a `TableRef`:

```rust
use toolu_orm_core::alias::TableRef;

let author = TableRef::aliased("users", "author");
let reviewer = TableRef::aliased("users", "reviewer");
let q = SelectBuilder::from_table(&author)
  .column_as(&author.column(&users::id), "author_id")
  .column_as(&reviewer.column(&users::id), "reviewer_id")
  .join(&reviewer, author.column(&users::org_id).equals(&reviewer.column(&users::org_id)));
```

## Distinct rows and grouped reports

`.distinct()` deduplicates the full projection before pagination.
`.group_by(&column)` and `.group_by_scalar(scalar)` add grouping keys;
`.having(expr)` filters groups, with repeated calls combined by `AND`.

```rust
use toolu_orm_core::expr::Scalar;

let q = UsersTable::select()
  .columns_qualified(&[&users::org_id])
  .column_scalar(Scalar::count_star(), "n")
  .group_by(&users::org_id)
  .having(Scalar::count_star().gt(Scalar::bind(1_i64)));
```

Aggregates include `count_star`, `count`, `count_distinct`, `sum`, `min`, `max`
and `avg`. Repeat an aggregate in `HAVING` instead of its output alias for
Postgres compatibility. See the
[grouping scenarios](https://github.com/Falconiere/toolu-orm/blob/main/docs/scenarios/distinct-and-grouping.md)
for NULL results and driver-specific aggregate types.

## Composing statements

`Cte::new(name, query)` and `.with(cte)` add a `WITH` member. A CTE can declare
`.columns(&[...])` and `.recursive()`. `.union(other)` deduplicates whole rows;
`.union_all(other)` keeps duplicates. A recursive query must terminate: `UNION`
only stops revisiting rows whose entire projection is unchanged.

```rust
use toolu_orm_query::select::Cte;

let active = Cte::new("active_users", UsersTable::select()
  .columns_raw(&["id"])
  .filter(users::status.eq("active")));
let q = SelectBuilder::from_table(active.table_ref())
  .columns_raw(&["id"])
  .with(active);
```

Set-operation arms contribute their projections, sources, joins and predicates;
their own ordering and pagination are ignored. Apply `order_by`, `limit` and
`offset` to the outer builder, and order by an output name with
`OrderBy::alias_asc("id")` or `alias_desc`.

A select also works inside `Scalar::subquery`, `Scalar::in_subquery`,
`Scalar::not_in_subquery`, `Expr::exists` and `Expr::not_exists`. Table-valued
functions can be sources through `TableRef::function`. Nested builders keep
one parameter sequence. A scalar subquery should project one column and at most
one row: Postgres rejects multiple rows, while SQLite uses the first. See the
[composition scenarios](https://github.com/Falconiere/toolu-orm/blob/main/docs/scenarios/query-composition.md)
for recursive walks, correlated subqueries and table functions.

## Rendering

Every builder renders to `(String, Vec<Value>)`:

| Method | Returns |
|---|---|
| `.to_sql_for(dialect)` | SQL for that dialect — `?N` for `Dialect::Sqlite`, `$N` for `Dialect::Postgres`. |
| `.to_sql()` | Same, for `Dialect::CURRENT` (the dialect implied by the active features). |
| `.to_count_sql_for(dialect)` / `.to_count_sql()` | Count matching rows; distinct, grouped and compound queries count a derived table. Drops outer ordering and pagination. |
| `.to_exists_sql_for(dialect)` / `.to_exists_sql()` | Check for a matching row or group, ignoring outer ordering and pagination. |
| `.to_first_row_sql_for(dialect)` / `.to_first_row_sql()` | Render the first-row form used by `fetch_one` and `fetch_optional`; see the limit behavior below. |

Rendering never touches the database, which is why the builders compile with any
feature combination and are straightforward to unit test.

## Executing

With exactly one driver feature active, the select gains its fetch methods.
They take an `Executor`: `libsql::Connection`, `rusqlite::Connection`,
`tokio_postgres::Client`, or the supported query transaction wrapper.
`toolu_orm_connection::RusqliteConnection` also implements `Executor` directly.
For libsql's connection wrapper, pass `conn.inner_conn()`.
See [Connections](../drivers/index.md) and [Transactions](transactions.md).

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

With no limit or a nonnegative limit, `fetch_one` and `fetch_optional` send the
bounded form of the query, so the database returns **at most one row** and at
most one row is ever decoded —
a match of ten thousand rows still costs one decode, and a row further down the
result set that fails to decode cannot fail an otherwise valid first row.
Filters, `ORDER BY` and `OFFSET` are preserved; an explicit `.limit(0)` still
yields no row, and an explicit positive limit still yields that page's first
row. Use `fetch_all` when you want the whole result set.

An explicitly negative limit is preserved: SQLite treats it as unbounded,
so these methods can decode every matching row before returning the first;
Postgres rejects it.

`select_for::<T>()` initializes the column list from `T::REQUIRED_COLUMNS`.
It returns an ordinary `SelectBuilder`, so use the same row type when fetching
and keep any later projection changes compatible with that type. You can also
set the projection explicitly with `select()` and `columns_raw`.

On the rusqlite driver these methods are synchronous — same names, no `.await`.

> Multiple driver features on `toolu-orm-query` disable the executor: the
> builders and `to_sql_for` still work, `fetch_*` is not compiled. See
> [Installation](../getting-started/installation.md).
