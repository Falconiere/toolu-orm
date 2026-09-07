# toolu-orm

Standalone Rust ORM: schema-driven migrations, type-safe query builders, proc macros for table definitions, and pluggable database drivers (libsql for Turso/remote SQLite, rusqlite for embedded SQLite, tokio-postgres for Postgres).

## Crates

| Crate | Type | Description |
|-------|------|-------------|
| `toolu-orm-core` | Library | Core types: `ColumnType`, `TableDef`, `Value`, `Expr`, `Column<T>`, `Snapshot`, `Journal`, diff engine, dialects |
| `toolu-orm-macros` | Proc-macro | `#[table]`, `#[derive(FromRow)]`, `#[derive(ColumnEnum)]` code generation |
| `toolu-orm-query` | Library | `SelectBuilder`, `InsertBuilder`, `UpdateBuilder`, `DeleteBuilder`, relational queries, `Executor` trait, transactions |
| `toolu-orm-connection` | Library | `Database` / `DbConnection` wrappers for libsql, rusqlite, and a Postgres pool |
| `toolu-orm-cli` | Library | Migration generation (schema diff), application, and status checking |

## Layout

```
toolu-orm/
└── crates/
    ├── orm-core/        # Foundation: types, schema, snapshot, diff, SQL generation
    ├── orm-macros/      # Proc macros: #[table], #[derive(FromRow)], #[derive(ColumnEnum)]
    ├── orm-query/       # Query builders: Select, Insert, Update, Delete + Executor
    ├── orm-connection/  # Driver connections: libsql, rusqlite, postgres
    └── orm-cli/         # Migrations: run_generate(), run_migrate(), get_status()
```

Dependency graph:

```
orm-macros (proc-macro)  → orm-core
orm-query (builders)     → orm-core
orm-connection (drivers) → orm-core
orm-cli (migrations)     → orm-core, orm-connection
```

## Driver features

Every crate exposes the same feature names, forwarded down to `toolu-orm-core`:

| Feature | Driver | Mode |
|---------|--------|------|
| `libsql` | [libsql](https://crates.io/crates/libsql) | async, local file / in-memory / Turso remote |
| `rusqlite` | [rusqlite](https://crates.io/crates/rusqlite) (bundled) | sync, wrapped in `spawn_blocking` |
| `postgres` | [tokio-postgres](https://crates.io/crates/tokio-postgres) + deadpool | async pool with rustls |

`toolu-orm-core` and `toolu-orm-cli` default to `libsql`; the other crates have no default driver. Activate the driver features you need on every crate you depend on, for example:

```toml
[dependencies]
toolu-orm-core = { version = "0.1", default-features = false, features = ["libsql", "postgres"] }
toolu-orm-macros = { version = "0.1", features = ["libsql", "postgres"] }
toolu-orm-query = { version = "0.1", features = ["libsql", "postgres"] }
toolu-orm-connection = { version = "0.1", features = ["libsql", "postgres"] }
toolu-orm-cli = { version = "0.1", default-features = false, features = ["libsql", "postgres"] }
```

The `FromRow` trait changes shape with the active drivers. `#[derive(FromRow)]` currently emits the `postgres` + `libsql` shape (`from_pg_row` plus a `from_libsql_row` stub), so the derive compiles only when `toolu-orm-core` has both `postgres` and `libsql` enabled. Hand-written `FromRow` impls work with any driver set.

## Migrations

- Migration files live in a directory and are applied in name order.
- Multi-statement files use the `--> statement-breakpoint` separator.
- Snapshots are JSON-serialized schema state; `run_generate` diffs the current schema against the last snapshot.
- The journal records migration order plus SHA256 hashes for integrity; `get_status` reports pending and drifted entries.

## Development

Toolchain is pinned in `rust-toolchain.toml`; tests run with [cargo-nextest](https://nexte.st).

```sh
# Format
cargo fmt --all -- --check

# Default-feature lane (libsql)
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace

# Postgres lane (also compiles the FromRow derive suites)
cargo clippy -p toolu-orm-core -p toolu-orm-macros -p toolu-orm-query -p toolu-orm-connection -p toolu-orm-cli --features postgres --all-targets -- -D warnings
cargo nextest run -p toolu-orm-core -p toolu-orm-macros -p toolu-orm-query -p toolu-orm-connection -p toolu-orm-cli --features postgres
```

All tests run against in-memory libsql databases or pure SQL-generation fixtures; no external database is required.

## License

MIT. See [LICENSE](LICENSE).
