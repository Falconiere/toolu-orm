# toolu-orm-query

Type-safe SQL query builders for toolu-orm. Select, Insert, Update, and Delete
builders render SQLite, Postgres, or Lance SQL without a database connection.

## Driver features

No driver is enabled by default. Enable exactly one of `libsql`, `rusqlite`, or
`postgres` for legacy execution and row fetching. libsql and Postgres are async;
rusqlite is synchronous. With zero or multiple driver features, builders and
explicit `to_sql_for(Dialect::...)` rendering remain available, but legacy
executors and fetch methods are not compiled. The `lancedb` feature can coexist
with other drivers for rendering; it does not provide a Lance `Executor` in this
crate.

`InsertBuilder`, `UpdateBuilder`, and `DeleteBuilder` always provide async
`execute_on(&impl DbConnection) -> Result<u64, DbError>`. This shared write API
renders using the connection's runtime dialect, works with any feature set, and
forwards parameters to the connection. It does not fetch `RETURNING` rows or
perform database capability checks; `.execute()` and fetch methods keep their
legacy feature gates.

The `sqlite-vec` feature enables `rusqlite` and the sqlite-vec registration
dependency. The query API uses `toolu-orm-core` types: `Expr`, `Scalar`,
`Column<T>`, `TableRef`, and `Value`.

## Architecture

```
src/
├── lib.rs                       # Module exports
├── select/                      # SelectBuilder, projections, joins, pagination
│   ├── grouping.rs              # DISTINCT, GROUP BY, HAVING
│   ├── cte.rs                   # WITH / WITH RECURSIVE
│   ├── compound.rs              # UNION / UNION ALL
│   ├── executor_fetch/          # Feature-gated fetch/count/exists methods
│   └── relational/              # Relational SQL and JSON/binary decoding
├── insert/                      # VALUES / SELECT, ON CONFLICT, RETURNING
├── update.rs                    # UpdateBuilder (set, set_expr, set_scalar, filter)
├── delete.rs                    # DeleteBuilder (filter)
├── executor/                    # Driver-specific Executor traits and implementations
├── transaction.rs               # libsql run_transaction closure API
├── relational_builder.rs        # RelationalQuery tuple shape
├── where_clause.rs              # Filter macro + WHERE/HAVING generation
├── error.rs                     # QueryError
└── exec_helpers.rs              # Execute and RETURNING fetch macros
```

## Usage

```rust
use toolu_orm_query::select::SelectBuilder;
use toolu_orm_core::alias::TableRef;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::query_column::CommonOps;

// Assuming #[table(name = "users")] generated the users column module.
let (sql, params) = SelectBuilder::new("users")
    .columns_typed(&[&users::id, &users::email])
    .filter(users::email.eq("alice@example.com"))
    .order_by(users::created_at.desc())
    .limit(10)
    .to_sql_for(Dialect::Sqlite);

// Aliased tables: older/newer users sharing an email (a self-join).
let older = TableRef::aliased("users", "older");
let newer = TableRef::aliased("users", "newer");
let (sql, params) = SelectBuilder::from_table(&older)
    .column_as(&older.column(&users::id), "older_id")
    .column_as(&newer.column(&users::id), "newer_id")
    .join(
        &newer,
        older
            .column(&users::email)
            .equals(&newer.column(&users::email))
            .and(
                older
                    .column(&users::created_at)
                    .less_than(&newer.column(&users::created_at)),
            ),
    )
    .to_sql_for(Dialect::Sqlite);
```

Set an explicit projection before fetching: `SelectBuilder::new` starts with an
empty list. Generated `UsersTable::select_for::<Row>()` fills it from
`Row::REQUIRED_COLUMNS`. Simple `count` and `exists` queries generate their own
projections. Distinct/grouped/compound counts and compound existence queries
preserve the projection, so those forms still need a valid select list. Both
helpers ignore outer ordering and pagination.

With the `libsql` feature and a raw `libsql::Connection` named `conn`:

```rust
use toolu_orm_query::insert::{InsertBuilder, OnConflict};

let rows = InsertBuilder::new("users")
    .set(&users::id, "uuid-123")
    .set(&users::email, "alice@example.com")
    .on_conflict(OnConflict::column(&users::id).set_excluded(&users::email))
    .execute(&conn)
    .await?;
```

The same calls work with a `tokio_postgres::Client` in the Postgres lane. For
rusqlite, pass a raw `rusqlite::Connection` or the connection crate's
`RusqliteConnection`, and omit `.await`.

Builder execution renders for the executor's `dialect()`: the built-in SQLite
executors use `?N`, and the Postgres client and transaction use `$N`. The
explicit `to_sql_for(Dialect::Lance)` path uses `?N` even when Postgres and
Lance features are both enabled. `to_sql()` remains a shorthand for
`Dialect::CURRENT`, which reflects compile-time features. A custom executor
can override `dialect()` when its runtime backend differs from `CURRENT`.
Lance supports offset-only SQL without SQLite's `LIMIT -1` prefix. `execute_on`
does not yet reject Lance `ON CONFLICT` or ordinary DML `RETURNING` before
execution; capability guards belong to issue #174. See [the Lance matrix](https://github.com/Falconiere/toolu-orm/blob/main/docs/scenarios/lancedb-sql-matrix.md).

`on_conflict` updates in place on either dialect. SQLite's `or_replace` deletes
and reinserts the conflicting row, which can reset omitted columns and cascade
deletions. Use `.returning(&column)` with `fetch_one`, `fetch_optional`, or
`fetch_all` to read inserted or updated values.

See the [query guides](https://github.com/Falconiere/toolu-orm/blob/main/website/src/queries/select.md) and the tested
scenarios for [scalar expressions](https://github.com/Falconiere/toolu-orm/blob/main/docs/scenarios/scalar-expressions.md),
[grouping](https://github.com/Falconiere/toolu-orm/blob/main/docs/scenarios/distinct-and-grouping.md),
[query composition](https://github.com/Falconiere/toolu-orm/blob/main/docs/scenarios/query-composition.md),
[reusable bindings](https://github.com/Falconiere/toolu-orm/blob/main/docs/scenarios/reusable-bound-parameters.md),
[upsert](https://github.com/Falconiere/toolu-orm/blob/main/docs/scenarios/upsert.md), and
[INSERT … SELECT](https://github.com/Falconiere/toolu-orm/blob/main/docs/scenarios/insert-select.md), and
[session-selected rendering](https://github.com/Falconiere/toolu-orm/blob/main/docs/scenarios/session-selected-dialect.md).

## Development

```sh
cargo build -p toolu-orm-query
cargo nextest run -p toolu-orm-query
cargo clippy -p toolu-orm-query -- -D warnings

# Run an execution lane as well; the default lane tests SQL rendering.
cargo nextest run -p toolu-orm-query --features libsql
cargo nextest run -p toolu-orm-query --features rusqlite,sqlite-vec
```
