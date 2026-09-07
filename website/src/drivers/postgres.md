# Postgres

```toml
toolu-orm-core       = { version = "0.1", default-features = false, features = ["postgres"] }
toolu-orm-macros     = { version = "0.1", features = ["postgres"] }
toolu-orm-query      = { version = "0.1", features = ["postgres"] }
toolu-orm-connection = { version = "0.1", features = ["postgres"] }
toolu-orm-cli        = { version = "0.1", default-features = false, features = ["postgres"] }
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
}).await?;

let conn = pg.connect().await?;   // PgConnection — DbConnection
```

`PgDatabase::init` builds the pool; `connect()` checks one connection out of it.
`ssl: true` enables rustls; leave it `false` for a local development database
that speaks plaintext.

`PgConfig::for_test("my_schema")` builds the configuration the test suites use —
`localhost:5433`, user and password `toolu` — with `TEST_DB_HOST`, `TEST_DB_PORT`,
`TEST_DB_USER` and `TEST_DB_PASSWORD` overriding any field. See
[Testing](../guides/testing.md).

## Dialect

Generate migrations with `Dialect::Postgres`. The difference from SQLite is
visible in the SQL:

- placeholders are `$1`, `$2`, … instead of `?1`, `?2`;
- `or_ignore` renders `ON CONFLICT DO NOTHING`, `or_replace` renders
  `ON CONFLICT (…) DO UPDATE SET … = EXCLUDED.…`;
- relational loads render `LEFT JOIN LATERAL` with `json_agg` /
  `json_build_array` instead of correlated `json_group_array` subqueries;
- column types use the Postgres names (`bigint`, `jsonb`, `varchar(n)`,
  `timestamp`, `uuid`, `serial`).

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

## Errors

Failures come back as `DbError::Query`, `DbError::Connection` or
`DbError::Pool`, carrying the driver message — including the SQLSTATE code for
constraint violations, which is what makes a unique-violation distinguishable
from a syntax error. See [Error handling](../guides/errors.md).
