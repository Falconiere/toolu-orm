# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### `toolu-orm-macros`

#### Added
- *(core,macros)* add partial index where clauses ([#67](https://github.com/Falconiere/toolu-orm/pull/67)) ([#78](https://github.com/Falconiere/toolu-orm/pull/78))
- *(macros)* add #[column(check = "...")] for raw CHECKs ([#66](https://github.com/Falconiere/toolu-orm/pull/66)) ([#75](https://github.com/Falconiere/toolu-orm/pull/75))

### `toolu-orm-core`

#### Added
- *(core,macros)* add partial index where clauses ([#67](https://github.com/Falconiere/toolu-orm/pull/67)) ([#78](https://github.com/Falconiere/toolu-orm/pull/78))

## [0.4.3](https://github.com/Falconiere/toolu-orm/compare/v0.4.2...v0.4.3) - 2025-09-12

### `toolu-orm-cli`

#### Fixed
- *(cli)* apply migration chunks with execute_batch ([#64](https://github.com/Falconiere/toolu-orm/pull/64)) ([#73](https://github.com/Falconiere/toolu-orm/pull/73))

### `toolu-orm-macros`

#### Fixed
- *(macros)* emit docs on generated public table items ([#69](https://github.com/Falconiere/toolu-orm/pull/69)) ([#72](https://github.com/Falconiere/toolu-orm/pull/72))

## [0.4.2](https://github.com/Falconiere/toolu-orm/compare/v0.4.1...v0.4.2) - 2026-09-12

### `toolu-orm-cli`

#### Fixed
- *(deps)* route every tokio pin through the workspace and narrow it to what each crate calls ([#68](https://github.com/Falconiere/toolu-orm/pull/68))

### `toolu-orm-query`

#### Fixed
- *(deps)* route every tokio pin through the workspace and narrow it to what each crate calls ([#68](https://github.com/Falconiere/toolu-orm/pull/68))

### `toolu-orm-connection`

#### Fixed
- *(deps)* route every tokio pin through the workspace and narrow it to what each crate calls ([#68](https://github.com/Falconiere/toolu-orm/pull/68))

## [0.4.1](https://github.com/Falconiere/toolu-orm/compare/v0.4.0...v0.4.1) - 2026-09-10

### `toolu-orm-cli`

#### Fixed
- *(deps)* route every libsql pin through the workspace dependency

### `toolu-orm-query`

#### Fixed
- *(deps)* route the tokio-postgres and postgres-types pins through the workspace
- *(deps)* route every libsql pin through the workspace dependency

### `toolu-orm-macros`

#### Fixed
- *(deps)* route every libsql pin through the workspace dependency

### `toolu-orm-connection`

#### Fixed
- *(deps)* route every libsql pin through the workspace dependency

#### Other
- *(deps)* explain why the libsql feature set is split between root and orm-connection

### `toolu-orm-core`

#### Fixed
- *(deps)* route the tokio-postgres and postgres-types pins through the workspace
- *(deps)* route every libsql pin through the workspace dependency

## [0.4.0](https://github.com/Falconiere/toolu-orm/compare/v0.3.0...v0.4.0) - 2026-09-10

### `toolu-orm-cli`

#### Other
- update Cargo.toml dependencies

### `toolu-orm-query`

#### Fixed
- *(core,query)* [**breaking**] bump rusqlite to 0.40 and route both crates through the workspace pin

### `toolu-orm-macros`

#### Other
- update Cargo.toml dependencies

### `toolu-orm-connection`

#### Other
- update Cargo.toml dependencies

### `toolu-orm-sqlite-vec-register`

#### Other
- update Cargo.toml dependencies

### `toolu-orm-core`

#### Fixed
- *(core,query)* [**breaking**] bump rusqlite to 0.40 and route both crates through the workspace pin

## [0.3.0](https://github.com/Falconiere/toolu-orm/compare/v0.2.0...v0.3.0) - 2026-09-10

### `toolu-orm-cli`

#### Added
- *(cli,query)* blocking migrate/status and RusqliteConnection Executor ([#47](https://github.com/Falconiere/toolu-orm/pull/47))
- *(cli)* embedded mark_applied / get_status ([#43](https://github.com/Falconiere/toolu-orm/pull/43))
- *(cli)* map a driver's `no such module: …` into `MigrateError::MissingExtension` ([#35](https://github.com/Falconiere/toolu-orm/pull/35))

#### Fixed
- *(cli)* [**breaking**] mark `MigrateError` non_exhaustive ([#50](https://github.com/Falconiere/toolu-orm/pull/50))

#### Breaking
- *(cli)* added `MigrateError::MissingExtension`

### `toolu-orm`

#### Added
- *(core)* rebuild FTS5 from external content on shape change ([#49](https://github.com/Falconiere/toolu-orm/pull/49))
- *(cli,query)* blocking migrate/status and RusqliteConnection Executor ([#47](https://github.com/Falconiere/toolu-orm/pull/47))
- *(cli)* embedded mark_applied / get_status ([#43](https://github.com/Falconiere/toolu-orm/pull/43))
- *(core)* model sqlite-vec vec0 virtual tables and Vector columns ([#35](https://github.com/Falconiere/toolu-orm/pull/35))

#### Other
- *(connection)* live sqlite-vec vec0 DDL and KNN ([#44](https://github.com/Falconiere/toolu-orm/pull/44))

### `toolu-orm-query`

#### Added
- *(core)* add Postgres pgvector distance query surface ([#48](https://github.com/Falconiere/toolu-orm/pull/48))
- *(cli,query)* blocking migrate/status and RusqliteConnection Executor ([#47](https://github.com/Falconiere/toolu-orm/pull/47))
- *(core)* add Postgres FTS @@ / ts_rank query surface ([#45](https://github.com/Falconiere/toolu-orm/pull/45))
- *(query)* add vec0 KNN form (MATCH + k + distance) ([#36](https://github.com/Falconiere/toolu-orm/pull/36))
- *(core)* add FTS5 MATCH and bm25 query surface ([#34](https://github.com/Falconiere/toolu-orm/pull/34))

#### Other
- *(connection)* live sqlite-vec vec0 DDL and KNN ([#44](https://github.com/Falconiere/toolu-orm/pull/44))

### `toolu-orm-macros`

#### Added
- *(core)* model sqlite-vec vec0 virtual tables and Vector columns ([#35](https://github.com/Falconiere/toolu-orm/pull/35))

### `toolu-orm-connection`

#### Other
- *(connection)* live sqlite-vec vec0 DDL and KNN ([#44](https://github.com/Falconiere/toolu-orm/pull/44))

### `toolu-orm-core`

#### Added
- *(core)* rebuild FTS5 from external content on shape change ([#49](https://github.com/Falconiere/toolu-orm/pull/49))
- *(core)* add Postgres pgvector distance query surface ([#48](https://github.com/Falconiere/toolu-orm/pull/48))
- *(core)* add Postgres FTS @@ / ts_rank query surface ([#45](https://github.com/Falconiere/toolu-orm/pull/45))
- *(core)* model sqlite-vec vec0 virtual tables and Vector columns ([#35](https://github.com/Falconiere/toolu-orm/pull/35))
- *(core)* add FTS5 MATCH and bm25 query surface ([#34](https://github.com/Falconiere/toolu-orm/pull/34))

#### Fixed
- *(core)* [**breaking**] mark public schema/error enums non_exhaustive (`DbCoreError`, `Operation`, `ColumnChange`, `ColumnType`, `VectorElement`, `TableKind`) ([#50](https://github.com/Falconiere/toolu-orm/pull/50))

#### Breaking
- *(core)* added `Operation::RecreateFts5FromContent`, `ColumnType::Vector`, and several `DbCoreError` variants (downstream exhaustive matches must add arms or a wildcard)

## [0.2.0](https://github.com/Falconiere/toolu-orm/compare/v0.1.2...v0.2.0) - 2026-09-08

### `toolu-orm-cli`

#### Added
- *(core)* model virtual tables with TableKind and FTS5 builders ([#28](https://github.com/Falconiere/toolu-orm/pull/28))
- *(connection)* add a blocking DbConnection trait for rusqlite ([#29](https://github.com/Falconiere/toolu-orm/pull/29))
- *(cli)* apply migrations embedded in the binary with include_str! ([#30](https://github.com/Falconiere/toolu-orm/pull/30))
- *(cli)* baseline an existing database with mark_applied ([#25](https://github.com/Falconiere/toolu-orm/pull/25))

#### Fixed
- *(macros)* make #[derive(FromRow)] follow the active driver set ([#31](https://github.com/Falconiere/toolu-orm/pull/31))
- *(macros)* emit absolute, consumer-resolved crate paths ([#26](https://github.com/Falconiere/toolu-orm/pull/26))

### `toolu-orm`

#### Added
- *(core)* model virtual tables with TableKind and FTS5 builders ([#28](https://github.com/Falconiere/toolu-orm/pull/28))
- *(connection)* add a blocking DbConnection trait for rusqlite ([#29](https://github.com/Falconiere/toolu-orm/pull/29))
- *(cli)* apply migrations embedded in the binary with include_str! ([#30](https://github.com/Falconiere/toolu-orm/pull/30))
- *(cli)* baseline an existing database with mark_applied ([#25](https://github.com/Falconiere/toolu-orm/pull/25))

#### Fixed
- *(macros)* make #[derive(FromRow)] follow the active driver set ([#31](https://github.com/Falconiere/toolu-orm/pull/31))
- *(macros)* emit absolute, consumer-resolved crate paths ([#26](https://github.com/Falconiere/toolu-orm/pull/26))

### `toolu-orm-query`

#### Added
- *(core)* model virtual tables with TableKind and FTS5 builders ([#28](https://github.com/Falconiere/toolu-orm/pull/28))
- *(connection)* add a blocking DbConnection trait for rusqlite ([#29](https://github.com/Falconiere/toolu-orm/pull/29))
- *(cli)* apply migrations embedded in the binary with include_str! ([#30](https://github.com/Falconiere/toolu-orm/pull/30))
- *(cli)* baseline an existing database with mark_applied ([#25](https://github.com/Falconiere/toolu-orm/pull/25))

#### Fixed
- *(macros)* make #[derive(FromRow)] follow the active driver set ([#31](https://github.com/Falconiere/toolu-orm/pull/31))
- *(macros)* emit absolute, consumer-resolved crate paths ([#26](https://github.com/Falconiere/toolu-orm/pull/26))

### `toolu-orm-macros`

#### Added
- *(core)* model virtual tables with TableKind and FTS5 builders ([#28](https://github.com/Falconiere/toolu-orm/pull/28))
- *(connection)* add a blocking DbConnection trait for rusqlite ([#29](https://github.com/Falconiere/toolu-orm/pull/29))
- *(cli)* apply migrations embedded in the binary with include_str! ([#30](https://github.com/Falconiere/toolu-orm/pull/30))
- *(cli)* baseline an existing database with mark_applied ([#25](https://github.com/Falconiere/toolu-orm/pull/25))

#### Fixed
- *(macros)* make #[derive(FromRow)] follow the active driver set ([#31](https://github.com/Falconiere/toolu-orm/pull/31))
- *(macros)* emit absolute, consumer-resolved crate paths ([#26](https://github.com/Falconiere/toolu-orm/pull/26))

### `toolu-orm-connection`

#### Added
- *(core)* model virtual tables with TableKind and FTS5 builders ([#28](https://github.com/Falconiere/toolu-orm/pull/28))
- *(connection)* add a blocking DbConnection trait for rusqlite ([#29](https://github.com/Falconiere/toolu-orm/pull/29))
- *(cli)* apply migrations embedded in the binary with include_str! ([#30](https://github.com/Falconiere/toolu-orm/pull/30))
- *(cli)* baseline an existing database with mark_applied ([#25](https://github.com/Falconiere/toolu-orm/pull/25))
- *(connection)* adopt a pre-configured rusqlite connection ([#24](https://github.com/Falconiere/toolu-orm/pull/24))

#### Fixed
- *(macros)* make #[derive(FromRow)] follow the active driver set ([#31](https://github.com/Falconiere/toolu-orm/pull/31))
- *(macros)* emit absolute, consumer-resolved crate paths ([#26](https://github.com/Falconiere/toolu-orm/pull/26))

### `toolu-orm-core`

#### Added
- *(core)* model virtual tables with TableKind and FTS5 builders ([#28](https://github.com/Falconiere/toolu-orm/pull/28))
- *(connection)* add a blocking DbConnection trait for rusqlite ([#29](https://github.com/Falconiere/toolu-orm/pull/29))
- *(cli)* apply migrations embedded in the binary with include_str! ([#30](https://github.com/Falconiere/toolu-orm/pull/30))
- *(cli)* baseline an existing database with mark_applied ([#25](https://github.com/Falconiere/toolu-orm/pull/25))

#### Fixed
- *(macros)* make #[derive(FromRow)] follow the active driver set ([#31](https://github.com/Falconiere/toolu-orm/pull/31))
- *(macros)* emit absolute, consumer-resolved crate paths ([#26](https://github.com/Falconiere/toolu-orm/pull/26))

#### Added
- *(orm-cli)* `migrate::run_migrate_embedded` applies migrations from a
  compile-time `&[EmbeddedMigration]` — SQL baked in with `include_str!` —
  so a single-binary distribution needs no migrations directory on the target
  machine ([#16](https://github.com/Falconiere/toolu-orm/issues/16)). It shares
  the hash check, transaction boundary and rollback with `run_migrate`, so the
  two sources are interchangeable; a repeated name is rejected with the new
  `MigrateError::DuplicateMigration`, and `EmbeddedMigration::verify_hash`
  checks a list without a database.
- *(orm-cli)* `migrate::mark_applied` and `migrate::mark_applied_through` record
  journal entries as applied without executing their SQL, so a database whose
  schema was built by a previous migration system can adopt toolu-orm without
  re-running every migration ([#14](https://github.com/Falconiere/toolu-orm/issues/14)).
  Hashes come from `_journal.json`, a name absent from the journal is rejected
  with the new `MigrateError::NotInJournal`, and already-recorded names are
  skipped.
- *(core)* `toolu_orm_core::serde` and `toolu_orm_core::serde_json` re-exports,
  so generated view structs and `Relational` impls reach serde without the
  consumer depending on it.
- *(ci)* `scripts/check-derive-matrix.sh` compiles `#[derive(FromRow)]` against
  all eight driver combinations. The four CI lanes give `toolu-orm-core` only
  four of them, so half of `impl_derived_from_row!`'s definitions were
  unreachable from the gate; a typo in one would have shipped silently
  ([#17](https://github.com/Falconiere/toolu-orm/issues/17)).

#### Fixed
- *(macros)* `#[derive(FromRow)]` expands to the trait shape `toolu-orm-core`
  actually compiled, so it works on every driver combination — including all
  three single-driver builds, which previously failed to compile and forced a
  hand-written `FromRow` for every row struct
  ([#17](https://github.com/Falconiere/toolu-orm/issues/17)). The derive emits
  one decoder per driver and hands them to the new
  `toolu_orm_core::impl_derived_from_row!`, whose eight definitions are
  `#[cfg]`-gated on `toolu-orm-core`'s own features; a `macro_rules!`
  definition is compiled with its defining crate's features, so this needs no
  build script and no feature flags on the consumer. libsql and rusqlite now
  get real decoders, retiring the `from_libsql_row` stub that returned
  `"<Type> is only decoded from Postgres rows"`, and `#[from_row(with = "…")]`
  applies on every driver rather than Postgres alone.
- *(macros)* the proc macros emit absolute paths resolved from the consuming
  crate's `Cargo.toml` (`proc-macro-crate`): `::toolu_orm::core::…` when
  `toolu-orm` is the dependency, `::toolu_orm_core::…` when the crates are
  named directly, honouring a Cargo rename. `#[table]`, `#[view]`,
  `#[derive(FromRow)]`, `#[derive(Relational)]` and `#[derive(ColumnEnum)]` now
  all compile with `toolu-orm` as the only dependency — previously the
  companion column module failed with `error[E0433]: cannot find module or
  crate toolu_orm_core`, which no import could fix ([#15]).
- *(macros)* `#[derive(ColumnEnum)]` reads `rename_all` even when another serde
  option precedes it, so `#[serde(crate = "…", rename_all = "…")]` renames
  variants as written.

#### Changed
- `toolu_orm::prelude` is now a convenience rather than a requirement; it keeps
  exporting the macros, `toolu_orm_core`, `toolu_orm_query` and the active
  driver crate. The README and the docs site no longer ask facade users to add
  `toolu-orm-core` as a second dependency.

[#15]: https://github.com/Falconiere/toolu-orm/issues/15

## [0.1.2](https://github.com/Falconiere/toolu-orm/compare/v0.1.1...v0.1.2) - 2026-09-07

### `toolu-orm-cli`

#### Fixed
- *(ci)* keep the Pages scopes out of the pull-request build

#### Other
- link the facade macro-path caveat to issue #15
- put the README quickstart on the facade it now installs
- correct the facade install section in the README
- fix the README quickstart and the FromRow/Executor description
- mdBook documentation site with GitHub Pages deploy

### `toolu-orm`

#### Fixed
- *(ci)* keep the Pages scopes out of the pull-request build

#### Other
- link the facade macro-path caveat to issue #15
- put the README quickstart on the facade it now installs
- correct the facade install section in the README
- fix the README quickstart and the FromRow/Executor description
- mdBook documentation site with GitHub Pages deploy

### `toolu-orm-query`

#### Fixed
- *(ci)* keep the Pages scopes out of the pull-request build

#### Other
- link the facade macro-path caveat to issue #15
- put the README quickstart on the facade it now installs
- correct the facade install section in the README
- fix the README quickstart and the FromRow/Executor description
- mdBook documentation site with GitHub Pages deploy

### `toolu-orm-macros`

#### Fixed
- *(ci)* keep the Pages scopes out of the pull-request build

#### Other
- link the facade macro-path caveat to issue #15
- put the README quickstart on the facade it now installs
- correct the facade install section in the README
- fix the README quickstart and the FromRow/Executor description
- mdBook documentation site with GitHub Pages deploy

### `toolu-orm-connection`

#### Fixed
- *(ci)* keep the Pages scopes out of the pull-request build

#### Other
- link the facade macro-path caveat to issue #15
- put the README quickstart on the facade it now installs
- correct the facade install section in the README
- fix the README quickstart and the FromRow/Executor description
- mdBook documentation site with GitHub Pages deploy

### `toolu-orm-core`

#### Fixed
- *(ci)* keep the Pages scopes out of the pull-request build

#### Other
- link the facade macro-path caveat to issue #15
- put the README quickstart on the facade it now installs
- correct the facade install section in the README
- fix the README quickstart and the FromRow/Executor description
- mdBook documentation site with GitHub Pages deploy

## [0.1.1](https://github.com/Falconiere/toolu-orm/compare/v0.1.0...v0.1.1) - 2026-09-07

### `toolu-orm-cli`

#### Added
- add the toolu-orm facade crate

### `toolu-orm`

#### Added
- add the toolu-orm facade crate
- extract toolu-orm from yamless-orm

#### Other
- *(orm)* apply rustfmt to the facade crate
- scenario pages and testing guide
- rewrite README as a full project guide

### `toolu-orm-query`

#### Added
- add the toolu-orm facade crate

### `toolu-orm-macros`

#### Added
- add the toolu-orm facade crate

### `toolu-orm-connection`

#### Added
- add the toolu-orm facade crate

### `toolu-orm-core`

#### Added
- add the toolu-orm facade crate

## [0.1.0](https://github.com/Falconiere/toolu-orm/releases/tag/v0.1.0) - 2026-09-07

### `toolu-orm-cli`

#### Added
- extract toolu-orm from yamless-orm

#### Fixed
- address PR #4 review feedback and the CI docs check
- *(cli)* skip comment-only chunks when applying a migration

#### Other
- correct scenario pages, module docs, and the stale expr.rs reference
- cover empty not_in on libsql and Postgres and ON DELETE CASCADE on Postgres
- scenario pages and testing guide
- *(cli)* migration loop, failure modes, and Postgres loop
- rewrite README as a full project guide
- automate crates.io releases with release-plz

### `toolu-orm-connection`

#### Added
- extract toolu-orm from yamless-orm

#### Other
- scenario pages and testing guide
- *(connection)* rusqlite lane and live Postgres suites
- rewrite README as a full project guide
- automate crates.io releases with release-plz

### `toolu-orm-query`

#### Added
- extract toolu-orm from yamless-orm

#### Fixed
- address PR #4 review feedback and the CI docs check

#### Other
- cover empty not_in on libsql and Postgres and ON DELETE CASCADE on Postgres
- scenario pages and testing guide
- *(query)* revive libsql suites and add per-driver scenario suites
- rewrite README as a full project guide
- automate crates.io releases with release-plz

### `toolu-orm-macros`

#### Added
- extract toolu-orm from yamless-orm

#### Other
- scenario pages and testing guide
- *(macros)* trybuild compile-fail cases
- rewrite README as a full project guide
- automate crates.io releases with release-plz

### `toolu-orm-core`

#### Added
- extract toolu-orm from yamless-orm

#### Fixed
- *(core)* render empty in_list as a constant instead of IN ()
- *(core)* render foreign keys on non-strict tables

#### Other
- correct scenario pages, module docs, and the stale expr.rs reference
- scenario pages and testing guide
- *(core)* expr offsets, legacy snapshot, rename resolver
- rewrite README as a full project guide
- automate crates.io releases with release-plz
