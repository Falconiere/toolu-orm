# toolu-orm

Standalone Rust ORM: schema-driven migrations, type-safe query builders, proc macros, drivers for libsql, rusqlite, and Postgres.

## Workspace
- Virtual workspace; five crates under `crates/`: orm-core, orm-macros, orm-query, orm-connection, orm-cli.
- orm-core is the foundation; every other crate depends on it. orm-cli also depends on orm-connection.
- orm-macros is a proc-macro crate and can only export proc macros.
- Toolchain pinned in `rust-toolchain.toml`. Lints live in the root `Cargo.toml` (`[workspace.lints]`) and `clippy.toml`; every crate inherits them with `[lints] workspace = true`.

## Driver features
- Features `libsql`, `rusqlite`, `postgres` exist on every crate and forward to orm-core.
- Consumers activate the drivers they need on every crate they depend on.
- `FromRow` changes shape per driver set. `#[derive(FromRow)]` emits the postgres+libsql shape, so suites that derive it are gated with `required-features = ["postgres"]` in `crates/orm-macros/Cargo.toml`.
- orm-core emits `DEP_TOOLU_ORM_CORE_HAS_*` build metadata (`links = "toolu_orm_core"`) so orm-cli's `build.rs` can see which features Cargo actually unified.

## Migrations
- Multi-statement migration files use the `--> statement-breakpoint` separator.
- Snapshots are JSON-serialized schema state used for diffing.
- The journal tracks migration order and SHA256 hashes for integrity.

## Rules
- No `.unwrap()`, `.expect()`, `panic!`, `unreachable!`, indexing with `[]` in `src/`; propagate with `?` / `ok_or`. Allowed in `tests/`.
- No `#[allow]` / `#[expect]`; fix the warning.
- No `#[cfg(test)]` in `src/`; tests live in each crate's `tests/` directory.
- Max 250 lines per file. Over that, split into a folder module whose `mod.rs` holds only `mod`, `pub use`, and `//!` docs.
- One concern per file; no `utils.rs` / `helpers.rs` / `common.rs` / `misc.rs`.
- Use `cargo nextest run`, never `cargo test`.

## Quality gate
```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace
cargo clippy -p toolu-orm-core -p toolu-orm-macros -p toolu-orm-query -p toolu-orm-connection -p toolu-orm-cli --features postgres --all-targets -- -D warnings
cargo nextest run -p toolu-orm-core -p toolu-orm-macros -p toolu-orm-query -p toolu-orm-connection -p toolu-orm-cli --features postgres
```
