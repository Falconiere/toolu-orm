# toolu-orm

Standalone Rust ORM: schema-driven migrations, type-safe query builders, proc macros, drivers for libsql, rusqlite, and Postgres.

## Workspace
- Virtual workspace; eight crates under `crates/`: orm-core, orm-macros, orm-query, orm-connection, orm-cli, orm (the `toolu-orm` facade), orm-facade-consumer, and `toolu-orm-sqlite-vec-register` (nested in `crates/orm-connection/`). The register crate owns the one `unsafe` call that registers sqlite-vec; it is published because orm-connection and orm-query depend on it behind their `sqlite-vec` feature, and `cargo publish` rejects any path-only dependency, optional or not.
- orm-core supplies the shared schema and query types. The facade depends on core, macros, query and connection; orm-cli depends on core and connection. orm-query also depends on connection under `rusqlite`. The standalone sqlite-vec register helper has no orm-core dependency, and the facade-consumer depends only on the facade.
- orm-macros is a proc-macro crate and can only export proc macros.
- orm (`toolu-orm`) is a facade: only re-exports, no logic.
- orm-facade-consumer (`publish = false`) ships nothing: its only dependency is `toolu-orm`, so its tests compile under a real external consumer's extern prelude. Never add a second dependency to it — that is the whole test.
- Macro expansions emit absolute paths resolved with `proc-macro-crate` (`crates/orm-macros/src/paths.rs`): `::toolu_orm_core` for a direct dependent, `::toolu_orm::core` for a facade-only one. `toolu_orm::prelude` is a convenience, not a requirement.
- Toolchain pinned in `rust-toolchain.toml`. Lints live in the root `Cargo.toml` (`[workspace.lints]`) and `clippy.toml`; every crate inherits them with `[lints] workspace = true`.

## Driver features
- Features `libsql`, `rusqlite`, `postgres`, and `lancedb` exist on the facade, core, macros, query, connection, CLI and facade-consumer; the dependents forward them to orm-core. `lancedb` provides `LanceConnection::open(path)` for bundled DuckDB plus a caller-supplied pinned Lance extension, then `attach(existing_directory, namespace)` for local table lifecycle and persisted rows. It has no `DbConnection`, query executor, portable schema renderer, or row decoder; it does not change the existing three-driver `FromRow` shape. The sqlite-vec register helper has no driver features.
- Consumers activate the drivers they need on every crate they depend on. For libsql/rusqlite query execution, keep core and query on the same single driver: query's scalar decoders implement the single-driver `FromRow` shape.
- `FromRow` changes shape per driver set: one driver on orm-core gives `from_row(&Row)`; two or more give `from_pg_row` / `from_libsql_row` / `from_rusqlite_row`. `#[derive(FromRow)]` follows that shape — it emits one decoder per driver and hands all of them to `toolu_orm_core::impl_derived_from_row!`, whose eight definitions are `#[cfg]`-gated on orm-core's own features (`crates/orm-core/src/row/derived.rs`). Deriving it therefore never pins a suite to a lane. Hand-written impls stay supported and stay covered (orm-cli's `migrate/store/applied.rs`, orm-connection's rusqlite fixtures, orm-query's `integration_test`).
- orm-query uses `cfg_single_backend!` for executors and their execute/fetch builder methods only when exactly one implemented driver is active and `lancedb` is absent. Filter building remains available across feature combinations; libsql transactions have the same single-driver gate.
- Crates that only *call* `FromRow` never name one of its methods: they go through `toolu_orm_core::row::from_{postgres,libsql,rusqlite}_row`, whose `#[cfg]`s are evaluated while compiling orm-core and so read the unified set by construction. Selecting a method from a crate's own feature flags is a proxy, and it drifts the moment one crate forwards a driver feature to orm-core but not to its sibling (issue #124).
- orm-core emits `DEP_TOOLU_ORM_CORE_HAS_*` build metadata (`links = "toolu_orm_core"`) so orm-cli's `build.rs` can see which features Cargo actually unified. That mechanism is for crates that must *implement* `FromRow` for their own types (`orm-cli`'s `migrate/store/applied.rs`, `migrate/pragma_row.rs`) and so cannot delegate to a helper.

## Tests
- Test files are flat: `crates/<crate>/tests/<name>_test.rs`, until one reaches the 250-line cap; then it becomes `tests/<name>_test/` whose entry file is `main.rs` — never `mod.rs`, which cargo never compiles (`scripts/check-test-targets.sh`) — holding only `mod` declarations, `#[path]` fixture wiring and `//!` docs, with the binary's own setup in a local `support.rs`. The binary name does not change, but `cargo nextest list` then prints `<module>::<test>`, so `docs/scenarios` rows must be module-qualified. Shared setup lives in `tests/fixtures/*.rs` (no `#[test]` there) and is wired in with `#[path = "fixtures/<file>.rs"] pub mod <name>;` (`pub mod`, so unused fixture items do not trip `dead_code`; `#[allow]` is banned).
- Driver integration tests use real databases: in-memory libsql, in-memory rusqlite, or live Postgres from `docker-compose.test.yaml` (`docker compose -f docker-compose.test.yaml up -d --wait`, then `TEST_DB_PORT=5434`). SQL rendering, schema diffs and macro compilation also have database-free tests. Postgres tests own a schema each and fail hard when the server is absent; never skip.
- Every `[[test]]` target with `required-features` must have a CI lane that satisfies it (see Quality gate). Verify with `cargo nextest list` per lane, not by counting `#[test]`.
- Every test scenario has a page in `docs/scenarios/` with a `## Tests` table naming its tests. `scripts/check-scenario-docs.sh` fails when a listed test is missing or a test in a scenario binary is undocumented, so a new or renamed test means a doc update in the same change.

## Releases
- release-plz opens the release PR on push to main and publishes only when a release PR merges (`release_always = false`; the merged PR's head branch must start with `release-plz-`). A CI or publish fix that has to trigger the release itself must use a `release-plz-*` branch.

## Migrations
- Multi-statement migration files use the `--> statement-breakpoint` separator.
- Snapshots are JSON-serialized schema state used for diffing.
- The journal tracks migration order and SHA256 hashes for integrity.

## Rules
- No `.unwrap()`, `.expect()`, `panic!`, `unreachable!`, indexing with `[]` in `src/`; propagate with `?` / `ok_or`. `clippy.toml` permits `unwrap` and `expect` in tests; the panic and indexing lints still apply.
- No `#[allow]` / `#[expect]`; fix the warning.
- No `#[cfg(test)]` in `src/`; tests live in each crate's `tests/` directory.
- Max 250 lines per file. Over that, split into a folder module whose `mod.rs` holds only `mod`, `pub use`, and `//!` docs.
- One concern per file; no `utils.rs` / `helpers.rs` / `common.rs` / `misc.rs`.
- Use `cargo nextest run`, never `cargo test`.

## Quality gate
Four existing-driver lanes plus five checks run in the `rust` CI job. The postgres lane needs the live server: `docker compose -f docker-compose.test.yaml up -d --wait` and `export TEST_DB_PORT=5434`. The lanes cover only four of the eight driver combinations, so `scripts/check-derive-matrix.sh` compiles the `FromRow` derive against all eight and `scripts/check-driver-matrix.sh` compiles the driver-dependent crates against all eight (orm-cli skips the unsupported no-driver case), plus six orm-connection builds where orm-core carries an extra driver. `scripts/check-test-targets.sh` fails when a test file is one no cargo target builds — a `tests/<dir>/` whose entry file is not `main.rs`, a flat test file in a crate with `autotests = false`, or a module file nothing declares. `scripts/check-file-length.sh` fails when any `*.rs` file under `crates/` — `src/` and `tests/` alike — is longer than 250 lines, which no clippy lint can express.
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
bash scripts/check-driver-matrix.sh
bash scripts/check-scenario-docs.sh
bash scripts/check-test-targets.sh
bash scripts/check-file-length.sh
```

The separate `lancedb-smoke` CI job runs `bash scripts/check-lancedb-smoke.sh`,
`bash scripts/check-lancedb-missing-extension.sh`, and
`bash scripts/check-lancedb-feature.sh` for the pinned real Lance probe,
production startup tests, a command-level missing-extension failure check, and
the optional dependency feature rule.
