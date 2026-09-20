# Testing

Driver integration tests run against real databases: in-memory libsql,
in-memory rusqlite, or live Postgres. SQL rendering, schema diff and macro
compilation tests also run without a database. Both matter: SQL assertions
check the exact output, while live execution checks that the engine accepts it
and that rows decode correctly.

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

`PgConfig::for_test("toolu")` selects the existing `toolu` database and reads `TEST_DB_HOST`, `TEST_DB_PORT`,
`TEST_DB_USER` and `TEST_DB_PASSWORD`, defaulting to `localhost:5433` with user
and password `toolu`. Test fixtures create separate schemas inside that database so tests do not
collide; `for_test` itself does not create a database or schema.

## Testing your own repositories

Two options, depending on what you want to prove:

- **Against the builders.** Take `&impl Executor` in your repository functions
  and hand them an in-memory driver connection in the test. For code using
  connection-level SQL methods or migrations, take `&impl DbConnection` (or
  `&impl DbConnectionBlocking`) and use the corresponding wrapper.
  Same code path as production, different database.
- **Against the SQL.** `to_sql_for(dialect)` returns `(String, Vec<Value>)` with
  no database involved, which is enough to assert on placeholder numbering or
  dialect differences in a unit test.

Both are useful; only the first tells you the statement runs.

## Running the suite

`cargo nextest run` — never `cargo test`. The full gate is four feature lanes
plus five checks, matching `.github/workflows/ci.yml`:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace

# postgres lane (needs the server above)
cargo clippy -p toolu-orm -p toolu-orm-core -p toolu-orm-macros -p toolu-orm-query \
  -p toolu-orm-connection -p toolu-orm-cli -p toolu-orm-facade-consumer --features postgres --all-targets -- -D warnings
cargo nextest run -p toolu-orm -p toolu-orm-core -p toolu-orm-macros -p toolu-orm-query \
  -p toolu-orm-connection -p toolu-orm-cli -p toolu-orm-facade-consumer --features postgres

# single-driver lanes: the executor and transaction code only compiles here
cargo clippy -p toolu-orm-query --features libsql --all-targets -- -D warnings
cargo nextest run -p toolu-orm-query --features libsql
cargo clippy -p toolu-orm-query --features rusqlite,sqlite-vec --all-targets -- -D warnings
cargo nextest run -p toolu-orm-query --features rusqlite,sqlite-vec
cargo clippy -p toolu-orm-connection --features rusqlite,sqlite-vec --all-targets -- -D warnings
cargo nextest run -p toolu-orm-connection --features rusqlite,sqlite-vec
cargo clippy -p toolu-orm-cli --no-default-features --features rusqlite --all-targets -- -D warnings
cargo nextest run -p toolu-orm-cli --no-default-features --features rusqlite

# the four lanes above give orm-core only four of the eight driver
# combinations; this compiles the FromRow derive against all eight
bash scripts/check-derive-matrix.sh
bash scripts/check-driver-matrix.sh
bash scripts/check-scenario-docs.sh
bash scripts/check-test-targets.sh
bash scripts/check-file-length.sh
```

The lanes exist because the code changes shape with the feature set: with more
than one driver active, `toolu-orm-query` does not compile its executor at all,
so a suite that calls `.execute()` has to run in a single-driver lane.
