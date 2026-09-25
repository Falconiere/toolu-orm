# Lance extension startup

**Scope:** Issue #158 opens embedded DuckDB with a previously provisioned Lance extension. It does not attach a Lance namespace or create tables during startup.

The production startup API accepts an explicit local extension path and verifies DuckDB `v1.5.5` and loaded Lance build `2f167ea`. A missing, directory, or corrupt file, or a non-UTF-8 path, returns `LanceStartupError::LanceDependencyUnavailable` before a target namespace exists. The successful test attaches a real temporary Lance namespace *after* startup and prepares a bound SELECT against a real row. Its extension path contains a quote, and the local file's bytes are unchanged after opening.

Paths containing backslashes or NUL are rejected before `LOAD` so SQL literal parsing cannot treat those characters as escapes.

Run `bash scripts/check-lancedb-smoke.sh` to provision the pinned artifact, compile the production `lancedb` lane, and run these tests.

## Tests

| Lane | Binary | Test |
|---|---|---|
| lancedb-smoke | lancedb_startup_test | pinned_extension_prepares_lance_sql_after_startup |
| lancedb-smoke | lancedb_startup_test | absent_extension_is_named_before_namespace_mutation |
| lancedb-smoke | lancedb_startup_test | directory_extension_path_is_named_before_namespace_mutation |
| lancedb-smoke | lancedb_startup_test | backslash_extension_path_is_rejected_before_loading |
| lancedb-smoke | lancedb_startup_test | corrupt_extension_is_named_before_namespace_mutation |
| lancedb-smoke | lancedb_startup_test | non_utf8_extension_path_is_named_before_namespace_mutation |
