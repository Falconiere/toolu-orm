# toolu-orm-cli

Migration tooling for toolu-orm. This is a library for generating SQL migrations, applying them with hash verification, baselining existing databases, and reporting migration status. It does not install a command-line executable.

## Stack

- **Database:** libsql, rusqlite, or Postgres through `toolu-orm-connection`
- **Schema:** toolu-orm-core (SchemaRegistry, Snapshot, Journal, diff)

The default feature is `libsql`. For another driver, use `default-features = false`
and enable `rusqlite` or `postgres`. Async APIs take `&impl DbConnection`;
blocking APIs take `&impl DbConnectionBlocking` (implemented by `RusqliteConnection`).

## Architecture

```
src/
├── lib.rs          # Module exports
├── generate.rs     # Migration generation from schema diffs
├── migrate/        # Directory/embedded runners, baselines, integrity, bookkeeping
└── status.rs       # Migration status reporting
```

## Usage

```rust
use toolu_orm_cli::{generate, migrate, status};
use toolu_orm_core::dialect::Dialect;

// `registry` is your SchemaRegistry; `conn` implements DbConnection.
// The output directory must already exist.
std::fs::create_dir_all("./migrations")?;
let filename = generate::run_generate(&registry, "./migrations", "add_users", Dialect::Sqlite)?;
// Some("0001_add_users.sql"), or None when the schema did not change.

// Apply migrations
let applied = migrate::run_migrate(&conn, "./migrations", Dialect::Sqlite).await?;

// Check status
let status = status::get_status(&conn, "./migrations", Dialect::Sqlite).await?;
println!("Applied: {:?}, Pending: {:?}", status.applied, status.pending);
```

Use `Dialect::Postgres` for Postgres. Generation reads the latest surviving
journal-linked snapshot and writes SQL, a snapshot, and `_journal.json`.
Review the SQL: some dialect-specific operations render comments that require
manual migration SQL.

The runner validates already-applied history, checks each pending migration's
hash, and applies journal entries in order, one transaction per migration. With
an absent or empty journal, it falls back to sorted `.sql` files without hash
verification. Status scans filenames and does not validate hashes.

Additional APIs:

- `migrate::run_migrate_embedded` accepts a slice of `EmbeddedMigration` values
  whose SQL can use `include_str!`; `status::get_status_embedded` reports their status.
- `migrate::mark_applied` and `mark_applied_through` baseline journal entries
  without executing SQL. Their `_embedded` variants use an embedded list.
- Each runner, baseline, and status function has a `_blocking` variant. For
  embedded functions the suffix follows `_embedded`, such as
  `run_migrate_embedded_blocking`.

See the [migration guide](https://github.com/Falconiere/toolu-orm/blob/main/website/src/migrations/overview.md) and
[journal documentation](https://github.com/Falconiere/toolu-orm/blob/main/website/src/migrations/journal-snapshots.md) for
integrity errors and validation limits.

## Development

```sh
cargo build -p toolu-orm-cli
cargo nextest run -p toolu-orm-cli
cargo clippy -p toolu-orm-cli -- -D warnings
```
