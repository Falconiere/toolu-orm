# Postgres

```toml
toolu-orm-core       = { version = "0.9", default-features = false, features = ["postgres"] }
toolu-orm-macros     = { version = "0.9", features = ["postgres"] }
toolu-orm-query      = { version = "0.9", features = ["postgres"] }
toolu-orm-connection = { version = "0.9", features = ["postgres"] }
toolu-orm-cli        = { version = "0.9", default-features = false, features = ["postgres"] }
```

Backed by `tokio-postgres` with a `deadpool-postgres` pool and rustls TLS.

## Pool and connections

```rust
use toolu_orm_connection::{PgConfig, PgDatabase};

let pg = PgDatabase::init(&PgConfig {
  host: "localhost".into(),
  port: 5432,
  user: "app".into(),
  password: std::env::var("PGPASSWORD")?,
  dbname: "app".into(),
  max_connections: 10,
  ssl: true,
  checkout_timeout: Some(PgConfig::DEFAULT_CHECKOUT_TIMEOUT),
}).await?;

let conn = pg.connect().await?;   // PgConnection — DbConnection
```

`PgDatabase::init` builds the pool and performs an initial checkout to verify
connectivity; an unreachable database fails during initialization. `connect()`
checks a connection out, and dropping it returns it to the pool.
`ssl: true` enables rustls; leave it `false` for a local development database
that speaks plaintext.

`checkout_timeout` bounds how long `connect()` waits for a free pool slot
(deadpool's checkout **wait** timeout) — distinct from connection-creation or
recycle timeouts, which are separate deadpool settings this config does not
expose. `PgConfig` has no `Default` implementation: set this field explicitly.
`Some(PgConfig::DEFAULT_CHECKOUT_TIMEOUT)` is 5s and is used by `for_test`;
`None` opts out for an unbounded wait.

`PgConfig::for_test("my_database")` sets the **database name**. It uses
`localhost:5433`, user and password `toolu`, with `TEST_DB_HOST`, `TEST_DB_PORT`,
`TEST_DB_USER` and `TEST_DB_PASSWORD` overriding those four settings. It also
sets five pool connections and disables TLS. See
[Testing](../guides/testing.md).

`PgConnection` supports `DbConnection` methods but does not expose an
`Executor` for query builders. Builders use a `tokio_postgres::Client` you open
separately, or the query crate's `PgTransaction` shown below.

The pooled connection's `execute_sql` and `query_map` reuse prepared statements
per connection, including inside its transactions. An execution error evicts
that statement and is returned without a retry. The caches have no size bound;
`pg.clear_statement_caches()` clears them across the pool.

## Dialect

Generate migrations with `Dialect::Postgres`. The difference from SQLite is
visible in the SQL:

- placeholders are `$1`, `$2`, … instead of `?1`, `?2`;
- `or_ignore` renders `ON CONFLICT DO NOTHING`, `or_replace` renders
  `ON CONFLICT (…) DO UPDATE SET … = EXCLUDED.…` (or `DO NOTHING` when no
  non-target column was inserted); its target is `conflict_columns(...)` or,
  by default, the first inserted column;
- relational loads render `LEFT JOIN LATERAL` with `json_agg` /
  `json_build_array` instead of correlated `json_group_array` subqueries;
- column types use the Postgres names (`bigint`, `jsonb`, `varchar(n)`,
  `TIMESTAMPTZ`, `uuid`, `serial`).

## Transactions

```rust
use toolu_orm_query::executor::PgTransaction;

let tx = PgTransaction::new(client.transaction().await?);
InsertBuilder::new("users").set(&users::id, "u1").execute(&tx).await?;
tx.commit().await?;
```

`PgConnection::transaction()` returns the connection-layer wrapper, which
implements `DbConnection`. Both roll back when dropped without a `commit`. See
[Transactions](../queries/transactions.md).

## Row-level security

Policies are declared on the table (`#[policy(...)]`, `#[table(rls = …)]`) and
migrated with everything else; see [Row-level security](../schema/row-level-security.md).
The per-request context a policy reads is set on the connection-layer
transaction with bound parameters rather than SQL text:

```rust
let tx = conn.transaction().await?;
tx.set_local_config("app.tenant_id", "42").await?;   // set_config($1, $2, true)
// … queries on `tx` see only tenant 42's rows …
tx.commit().await?;                                  // the setting is gone
```

## Errors

Connection operations return `DbError`; transaction and row-decoding failures
have their own variants. `execute_sql` and `query_map` include severity, message,
and SQLSTATE for server errors. `execute_batch` stringifies the driver error
without that extra formatting. Builders using a raw client return
`QueryError::Driver`, which preserves the `tokio_postgres::Error`. See
[Error handling](../guides/errors.md).
