# toolu-orm

Standalone Rust ORM: schema-driven migrations, type-safe query builders, proc macros, drivers for libsql, rusqlite, and Postgres.

## Workspace
- Virtual workspace; seven crates under `crates/`: orm-core, orm-macros, orm-query, orm-connection, orm-cli, orm (the `toolu-orm` facade), and orm-facade-consumer.
- orm-core is the foundation; every other crate depends on it. orm-cli also depends on orm-connection.
- orm-macros is a proc-macro crate and can only export proc macros.
- orm (`toolu-orm`) is a facade: only re-exports, no logic.
- orm-facade-consumer (`publish = false`) ships nothing: its only dependency is `toolu-orm`, so its tests compile under a real external consumer's extern prelude. Never add a second dependency to it — that is the whole test.
- Macro expansions emit absolute paths resolved with `proc-macro-crate` (`crates/orm-macros/src/paths.rs`): `::toolu_orm_core` for a direct dependent, `::toolu_orm::core` for a facade-only one. `toolu_orm::prelude` is a convenience, not a requirement.
- Toolchain pinned in `rust-toolchain.toml`. Lints live in the root `Cargo.toml` (`[workspace.lints]`) and `clippy.toml`; every crate inherits them with `[lints] workspace = true`.

## Driver features
- Features `libsql`, `rusqlite`, `postgres` exist on every crate and forward to orm-core.
- Consumers activate the drivers they need on every crate they depend on.
- `FromRow` changes shape per driver set: one driver on orm-core gives `from_row(&Row)`; two or more give `from_pg_row` / `from_libsql_row` / `from_rusqlite_row`. `#[derive(FromRow)]` follows that shape — it emits one decoder per driver and hands all of them to `toolu_orm_core::impl_derived_from_row!`, whose eight definitions are `#[cfg]`-gated on orm-core's own features (`crates/orm-core/src/row/derived.rs`). Deriving it therefore never pins a suite to a lane. Hand-written impls stay supported and stay covered (orm-cli's `migrate/store.rs`, orm-connection's rusqlite fixtures, orm-query's `integration_test`).
- orm-query compiles its executor, transaction, and fetch code only when exactly one driver feature is active (`cfg_single_backend!`), which is why the libsql-only and rusqlite-only lanes exist.
- orm-core emits `DEP_TOOLU_ORM_CORE_HAS_*` build metadata (`links = "toolu_orm_core"`) so orm-cli's `build.rs` can see which features Cargo actually unified.

## Tests
- Test files are flat: `crates/<crate>/tests/<name>_test.rs`. Shared setup lives in `tests/fixtures/*.rs` (no `#[test]` there) and is wired in with `#[path = "fixtures/<file>.rs"] pub mod <name>;` (`pub mod`, so unused fixture items do not trip `dead_code`; `#[allow]` is banned).
- Every test runs against a real database: in-memory libsql, in-memory rusqlite, or the live Postgres from `docker-compose.test.yaml` (`docker compose -f docker-compose.test.yaml up -d --wait`, then `TEST_DB_PORT=5434`). Postgres tests own a schema each and fail hard when the server is absent; never skip.
- Every `[[test]]` target with `required-features` must have a CI lane that satisfies it (see Quality gate). Verify with `cargo nextest list` per lane, not by counting `#[test]`.
- Every test scenario has a page in `docs/scenarios/` with a `## Tests` table naming its tests. `scripts/check-scenario-docs.sh` fails when a listed test is missing or a test in a scenario binary is undocumented, so a new or renamed test means a doc update in the same change.

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
Four lanes plus two checks, exactly what `.github/workflows/ci.yml` runs. The postgres lane needs the live server: `docker compose -f docker-compose.test.yaml up -d --wait` and `export TEST_DB_PORT=5434`. The lanes cover only four of the eight driver combinations, so `scripts/check-derive-matrix.sh` compiles the `FromRow` derive against all eight.
```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace
cargo clippy -p toolu-orm -p toolu-orm-core -p toolu-orm-macros -p toolu-orm-query -p toolu-orm-connection -p toolu-orm-cli -p toolu-orm-facade-consumer --features postgres --all-targets -- -D warnings
cargo nextest run -p toolu-orm -p toolu-orm-core -p toolu-orm-macros -p toolu-orm-query -p toolu-orm-connection -p toolu-orm-cli -p toolu-orm-facade-consumer --features postgres
cargo clippy -p toolu-orm-query --features libsql --all-targets -- -D warnings
cargo nextest run -p toolu-orm-query --features libsql
cargo clippy -p toolu-orm-query --features rusqlite,sqlite-vec --all-targets -- -D warnings
cargo nextest run -p toolu-orm-query --features rusqlite,sqlite-vec
cargo clippy -p toolu-orm-connection --features rusqlite,sqlite-vec --all-targets -- -D warnings
cargo nextest run -p toolu-orm-connection --features rusqlite,sqlite-vec
cargo clippy -p toolu-orm-cli --no-default-features --features rusqlite --all-targets -- -D warnings
cargo nextest run -p toolu-orm-cli --no-default-features --features rusqlite
bash scripts/check-derive-matrix.sh
bash scripts/check-scenario-docs.sh
```
