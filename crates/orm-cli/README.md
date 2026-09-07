# toolu-orm-cli

Migration tooling for toolu-orm. Generates SQL migrations from schema diffs, applies pending migrations with hash verification, and reports migration status.

## Stack

- **Database:** libsql (async, Turso-compatible)
- **Schema:** toolu-orm-core (SchemaRegistry, Snapshot, Journal, diff)

## Architecture

```
src/
├── lib.rs          # Module exports
├── generate.rs     # Migration generation from schema diffs
├── migrate.rs      # Migration application + journal tracking
└── status.rs       # Migration status reporting
```

## Usage

```rust
use toolu_orm_cli::{generate, migrate, status};
use toolu_orm_core::schema::SchemaRegistry;

// Generate migration
let filename = generate::run_generate(&registry, "./migrations", "add_users")?;

// Apply migrations
let applied = migrate::run_migrate(&conn, "./migrations").await?;

// Check status
let status = status::get_status(&conn, "./migrations").await?;
println!("Applied: {:?}, Pending: {:?}", status.applied, status.pending);
```

## Development

```sh
cargo build -p toolu-orm-cli
cargo nextest run -p toolu-orm-cli
cargo clippy -p toolu-orm-cli -- -D warnings
```
