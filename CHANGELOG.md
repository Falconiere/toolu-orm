# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
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

### Fixed
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

### Changed
- `toolu_orm::prelude` is now a convenience rather than a requirement; it keeps
  exporting the macros, `toolu_orm_core`, `toolu_orm_query` and the active
  driver crate. The README and the docs site no longer ask facade users to add
  `toolu-orm-core` as a second dependency.

[#15]: https://github.com/Falconiere/toolu-orm/issues/15

## `toolu-orm-cli` - [0.1.2](https://github.com/Falconiere/toolu-orm/compare/v0.1.1...v0.1.2) - 2026-09-07

### Fixed
- *(ci)* keep the Pages scopes out of the pull-request build

### Other
- link the facade macro-path caveat to issue #15
- put the README quickstart on the facade it now installs
- correct the facade install section in the README
- fix the README quickstart and the FromRow/Executor description
- mdBook documentation site with GitHub Pages deploy

## `toolu-orm` - [0.1.2](https://github.com/Falconiere/toolu-orm/compare/toolu-orm-v0.1.1...toolu-orm-v0.1.2) - 2026-09-07

### Fixed
- *(ci)* keep the Pages scopes out of the pull-request build

### Other
- link the facade macro-path caveat to issue #15
- put the README quickstart on the facade it now installs
- correct the facade install section in the README
- fix the README quickstart and the FromRow/Executor description
- mdBook documentation site with GitHub Pages deploy

## `toolu-orm-query` - [0.1.2](https://github.com/Falconiere/toolu-orm/compare/toolu-orm-query-v0.1.1...toolu-orm-query-v0.1.2) - 2026-09-07

### Fixed
- *(ci)* keep the Pages scopes out of the pull-request build

### Other
- link the facade macro-path caveat to issue #15
- put the README quickstart on the facade it now installs
- correct the facade install section in the README
- fix the README quickstart and the FromRow/Executor description
- mdBook documentation site with GitHub Pages deploy

## `toolu-orm-macros` - [0.1.2](https://github.com/Falconiere/toolu-orm/compare/toolu-orm-macros-v0.1.1...toolu-orm-macros-v0.1.2) - 2026-09-07

### Fixed
- *(ci)* keep the Pages scopes out of the pull-request build

### Other
- link the facade macro-path caveat to issue #15
- put the README quickstart on the facade it now installs
- correct the facade install section in the README
- fix the README quickstart and the FromRow/Executor description
- mdBook documentation site with GitHub Pages deploy

## `toolu-orm-connection` - [0.1.2](https://github.com/Falconiere/toolu-orm/compare/toolu-orm-connection-v0.1.1...toolu-orm-connection-v0.1.2) - 2026-09-07

### Fixed
- *(ci)* keep the Pages scopes out of the pull-request build

### Other
- link the facade macro-path caveat to issue #15
- put the README quickstart on the facade it now installs
- correct the facade install section in the README
- fix the README quickstart and the FromRow/Executor description
- mdBook documentation site with GitHub Pages deploy

## `toolu-orm-core` - [0.1.2](https://github.com/Falconiere/toolu-orm/compare/toolu-orm-core-v0.1.1...toolu-orm-core-v0.1.2) - 2026-09-07

### Fixed
- *(ci)* keep the Pages scopes out of the pull-request build

### Other
- link the facade macro-path caveat to issue #15
- put the README quickstart on the facade it now installs
- correct the facade install section in the README
- fix the README quickstart and the FromRow/Executor description
- mdBook documentation site with GitHub Pages deploy

## `toolu-orm-cli` - [0.1.1](https://github.com/Falconiere/toolu-orm/compare/v0.1.0...v0.1.1) - 2026-09-07

### Added
- add the toolu-orm facade crate

## `toolu-orm` - [0.1.1](https://github.com/Falconiere/toolu-orm/compare/toolu-orm-v0.1.0...toolu-orm-v0.1.1) - 2026-09-07

### Added
- add the toolu-orm facade crate
- extract toolu-orm from yamless-orm

### Other
- *(orm)* apply rustfmt to the facade crate
- scenario pages and testing guide
- rewrite README as a full project guide

## `toolu-orm-query` - [0.1.1](https://github.com/Falconiere/toolu-orm/compare/toolu-orm-query-v0.1.0...toolu-orm-query-v0.1.1) - 2026-09-07

### Added
- add the toolu-orm facade crate

## `toolu-orm-macros` - [0.1.1](https://github.com/Falconiere/toolu-orm/compare/toolu-orm-macros-v0.1.0...toolu-orm-macros-v0.1.1) - 2026-09-07

### Added
- add the toolu-orm facade crate

## `toolu-orm-connection` - [0.1.1](https://github.com/Falconiere/toolu-orm/compare/toolu-orm-connection-v0.1.0...toolu-orm-connection-v0.1.1) - 2026-09-07

### Added
- add the toolu-orm facade crate

## `toolu-orm-core` - [0.1.1](https://github.com/Falconiere/toolu-orm/compare/toolu-orm-core-v0.1.0...toolu-orm-core-v0.1.1) - 2026-09-07

### Added
- add the toolu-orm facade crate

## `toolu-orm-cli` - [0.1.0](https://github.com/Falconiere/toolu-orm/releases/tag/v0.1.0) - 2026-09-07

### Added
- extract toolu-orm from yamless-orm

### Fixed
- address PR #4 review feedback and the CI docs check
- *(cli)* skip comment-only chunks when applying a migration

### Other
- correct scenario pages, module docs, and the stale expr.rs reference
- cover empty not_in on libsql and Postgres and ON DELETE CASCADE on Postgres
- scenario pages and testing guide
- *(cli)* migration loop, failure modes, and Postgres loop
- rewrite README as a full project guide
- automate crates.io releases with release-plz

## `toolu-orm-connection` - [0.1.0](https://github.com/Falconiere/toolu-orm/releases/tag/v0.1.0) - 2026-09-07

### Added
- extract toolu-orm from yamless-orm

### Other
- scenario pages and testing guide
- *(connection)* rusqlite lane and live Postgres suites
- rewrite README as a full project guide
- automate crates.io releases with release-plz

## `toolu-orm-query` - [0.1.0](https://github.com/Falconiere/toolu-orm/releases/tag/v0.1.0) - 2026-09-07

### Added
- extract toolu-orm from yamless-orm

### Fixed
- address PR #4 review feedback and the CI docs check

### Other
- cover empty not_in on libsql and Postgres and ON DELETE CASCADE on Postgres
- scenario pages and testing guide
- *(query)* revive libsql suites and add per-driver scenario suites
- rewrite README as a full project guide
- automate crates.io releases with release-plz

## `toolu-orm-macros` - [0.1.0](https://github.com/Falconiere/toolu-orm/releases/tag/v0.1.0) - 2026-09-07

### Added
- extract toolu-orm from yamless-orm

### Other
- scenario pages and testing guide
- *(macros)* trybuild compile-fail cases
- rewrite README as a full project guide
- automate crates.io releases with release-plz

## `toolu-orm-core` - [0.1.0](https://github.com/Falconiere/toolu-orm/releases/tag/v0.1.0) - 2026-09-07

### Added
- extract toolu-orm from yamless-orm

### Fixed
- *(core)* render empty in_list as a constant instead of IN ()
- *(core)* render foreign keys on non-strict tables

### Other
- correct scenario pages, module docs, and the stale expr.rs reference
- scenario pages and testing guide
- *(core)* expr offsets, legacy snapshot, rename resolver
- rewrite README as a full project guide
- automate crates.io releases with release-plz

## `toolu-orm-cli` - [0.1.0](https://github.com/Falconiere/toolu-orm/releases/tag/v0.1.0) - 2026-09-07

### Added
- extract toolu-orm from yamless-orm

### Other
- rewrite README as a full project guide
- automate crates.io releases with release-plz

## `toolu-orm-connection` - [0.1.0](https://github.com/Falconiere/toolu-orm/releases/tag/v0.1.0) - 2026-09-07

### Added
- extract toolu-orm from yamless-orm

### Other
- rewrite README as a full project guide
- automate crates.io releases with release-plz

## `toolu-orm-query` - [0.1.0](https://github.com/Falconiere/toolu-orm/releases/tag/v0.1.0) - 2026-09-07

### Added
- extract toolu-orm from yamless-orm

### Other
- rewrite README as a full project guide
- automate crates.io releases with release-plz

## `toolu-orm-macros` - [0.1.0](https://github.com/Falconiere/toolu-orm/releases/tag/v0.1.0) - 2026-09-07

### Added
- extract toolu-orm from yamless-orm

### Other
- rewrite README as a full project guide
- automate crates.io releases with release-plz

## `toolu-orm-core` - [0.1.0](https://github.com/Falconiere/toolu-orm/releases/tag/v0.1.0) - 2026-09-07

### Added
- extract toolu-orm from yamless-orm

### Other
- rewrite README as a full project guide
- automate crates.io releases with release-plz
