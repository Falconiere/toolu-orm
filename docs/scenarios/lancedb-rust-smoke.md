# Rust DuckDB–Lance smoke probe

**Scope:** Issue #146 proves a local Lance directory can be accessed through the Rust `duckdb` binding and the DuckDB Lance extension. The probe lives in an isolated Cargo package at `probes/lancedb/`; it does not expose an ORM adapter or claim the broader SQL capability matrix.

## Pinned dependency pair

| Component | Exact version or identity |
|---|---|
| Rust binding | `duckdb =1.10505.0`, with `bundled` |
| DuckDB engine | `v1.5.5`, checked at runtime |
| Lance core extension | build `2f167ea`, checked at runtime |
| Local platform | macOS 26.6.2 (build 25G83), `osx_arm64` |
| CI platform | Ubuntu 24.04, `linux_amd64` (`ubuntu-24.04` runner) |

The runner downloads the signed core extension from the DuckDB `v1.5.5` repository and verifies the compressed artifact before loading it:

| Platform | SHA-256 of `lance.duckdb_extension.gz` |
|---|---|
| `osx_arm64` | `9c4e51633101be4e51d6a790859a2464d115784eda767608708e23ea5e955b19` |
| `linux_amd64` | `cae58f5c0831454b44b973875d0d457bf96574b0c629875b732ef4578fb07080` |

The script rejects other platforms and reports `LanceDependencyUnavailable` if download, checksum, decompression, or load fails. It uses independent temporary directories for each run.

## What is proven

The real Rust test opens in-memory DuckDB, loads the pinned extension, and attaches a new local Lance namespace. It creates `items`, inserts `(1, 'persisted')`, drops the connection, opens a new connection, reattaches the same directory, and executes an explicitly prepared `SELECT label FROM lance_smoke.main.items WHERE id = ?` with `1_i64` bound. The returned label is `persisted`, and `items.lance` exists on disk.

The failure test passes a nonexistent extension file to the same loader. It receives an error naming `LanceDependencyUnavailable` and the missing file before any attach or table mutation, and no Lance dataset exists.

## Run

```sh
bash scripts/check-lancedb-smoke.sh
```

This command checks formatting and Clippy for the isolated package, downloads the platform artifact, verifies its SHA-256, checks the test names below and in the [SELECT/DML matrix](lancedb-sql-matrix.md) and [DDL, constraint, and search matrix](lancedb-ddl-constraints-search.md) against `cargo nextest list`, and runs the real probes. CI runs it in the `lancedb-smoke` job. The production connection belongs to later epic issues.

## Tests

| Lane | Binary | Test |
|---|---|---|
| lancedb-smoke | lancedb_smoke_test | bound_select_returns_inserted_row_after_reopen |
| lancedb-smoke | lancedb_smoke_test | missing_extension_fails_before_table_mutation |
