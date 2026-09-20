---
name: auditing-documentation
description: Use when auditing toolu-orm documentation against implementation or updating docs after API or feature changes.
metadata:
  toolu:
    origin: agent
    created: 2026-09-20T02:06:55Z
---
## When to Use

Use when auditing toolu-orm documentation against implementation or updating docs after an API or feature change.

## Procedure

1. Recall repo knowledge and read `CLAUDE.md` plus the affected crate's instructions. Inspect the worktree diff before editing.
2. Map claims across root/package READMEs, `website/src/`, `docs/scenarios/`, crate instructions and inline rustdoc. Preserve historical changelog entries.
3. Establish facts from Cargo manifests/metadata, public exports, macro parsers and expansions, SQL renderers, execution paths, and focused tests. Check CI commands against `.github/workflows/ci.yml`; documentation is not evidence for another document.
4. Correct related surfaces together. Explain implemented limits, driver differences and required feature sets. Record runtime defects separately rather than changing behavior during a docs-only audit.
5. Extract complete examples into a temporary external consumer with local path dependencies and exactly the documented direct dependencies. Use the pinned Rust toolchain; run database examples in a temporary working directory. Give each independent example its own database/migration directory.

## Pitfalls

- An mdBook build does not compile Rust examples. Workspace feature unification can hide an external consumer's missing dependencies.
- A successful default rustdoc build misses links in feature-gated executors. Check individual drivers too.
- The scenario checker verifies test names, not prose or runtime behavior. Folder-module tests need module-qualified names.
- Driver rendering and database execution are separate contracts. Inspect both before claiming parity or automatic migration support.

## Verification

- `mdbook build website` with the version pinned in `.github/workflows/pages.yml`.
- `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps`; also document query/connection separately with each single driver.
- `bash scripts/check-scenario-docs.sh` (compiles/lists all lanes; no database required).
- Check local Markdown links, fences and complete TOML/JSON examples; run corrected Rust examples and relevant existing `cargo nextest` suites.
- `cargo fmt --all -- --check`, `git diff --check`, test-target and Rust file-length checks. Confirm Rust changes are comments only when behavior changes were not requested.
- Report checks actually run and distinguish live Postgres execution from compile-only validation.
