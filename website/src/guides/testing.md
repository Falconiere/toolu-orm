# Testing

Every test in this project runs against a real database — in-memory libsql,
in-memory rusqlite, or a live Postgres. There are no mocked connections, because
the interesting failures (placeholder numbering, `ON CONFLICT` shape, JSON
aggregation, `NULL` decoding) only appear when a real engine parses the SQL.

## In-memory, per test

libsql and rusqlite give a fresh database in a few lines, fast enough to run one
per test:

```rust
async fn setup_db() -> Result<libsql::Connection, Box<dyn std::error::Error>> {
  let db = libsql::Builder::new_local(":memory:").build().await?;
  let conn = db.connect()?;
  conn.execute("CREATE TABLE users (id TEXT PRIMARY KEY, email TEXT NOT NULL)", ()).await?;
  Ok(conn)
}
```

Put shared setup in `tests/fixtures/<name>.rs` and wire it in with a path module
so it stays out of the test binary's namespace:

```rust
#[path = "fixtures/libsql_db.rs"]
pub mod db;
```

## Live Postgres

The Postgres suites need a server; they fail rather than skip when it is absent,
so a green run means the assertions actually executed.

```sh
docker compose -f docker-compose.test.yaml up -d --wait   # pgvector/pgvector:pg16 on localhost:5434
export TEST_DB_PORT=5434
```

`PgConfig::for_test("suite_name")` reads `TEST_DB_HOST`, `TEST_DB_PORT`,
`TEST_DB_USER` and `TEST_DB_PASSWORD`, defaulting to `localhost:5433` with user
and password `toolu`. Each suite owns a schema, so tests do not collide.

## Testing your own repositories

Two options, depending on what you want to prove:

- **Against the builders.** Take `&impl DbConnection` (or `&impl Executor`) in
  your repository functions and hand them an in-memory connection in the test.
  Same code path as production, different database.
- **Against the SQL.** `to_sql_for(dialect)` returns `(String, Vec<Value>)` with
  no database involved, which is enough to assert on placeholder numbering or
  dialect differences in a unit test.

Both are useful; only the first tells you the statement runs.

## Running the suite

`cargo nextest run` — never `cargo test`. The full gate is four feature lanes
plus a docs check, exactly what CI runs:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace

# postgres lane (needs the server above)
cargo clippy -p toolu-orm -p toolu-orm-core -p toolu-orm-macros -p toolu-orm-query \
  -p toolu-orm-connection -p toolu-orm-cli --features postgres --all-targets -- -D warnings
cargo nextest run -p toolu-orm -p toolu-orm-core -p toolu-orm-macros -p toolu-orm-query \
  -p toolu-orm-connection -p toolu-orm-cli --features postgres

# single-driver lanes: the executor and transaction code only compiles here
cargo nextest run -p toolu-orm-query --features libsql
cargo nextest run -p toolu-orm-query --features rusqlite,sqlite-vec
cargo nextest run -p toolu-orm-connection --features rusqlite,sqlite-vec

# the four lanes above give orm-core only four of the eight driver
# combinations; this compiles the FromRow derive against all eight
bash scripts/check-derive-matrix.sh

bash scripts/check-scenario-docs.sh
```

The lanes exist because the code changes shape with the feature set: with more
than one driver active, `toolu-orm-query` does not compile its executor at all,
so a suite that calls `.execute()` has to run in a single-driver lane.
