# LanceDB namespace and table lifecycle

**Scope:** Issue #159 adds local Lance directory attachment and table create, open, list, and drop operations on top of the loaded extension from #158. The public API is synchronous and does not yet implement `DbConnection`, portable schema rendering, or query builders.

`LanceConnection::open(extension_path)?.attach(existing_directory, "catalog")?` attaches one real local directory and selects that catalog with `USE`. Unqualified SQL through `LanceNamespace::connection()` addresses its tables. The caller creates an empty directory before the first attach. A fresh connection may attach the same directory and read persisted rows. The attachment consumes the loaded connection; an attach failure drops it without creating the directory or changing tables.

`create_table` accepts `LanceColumn` definitions with `BigInt` or `Varchar` types for this lifecycle slice. It refuses a duplicate table without replacing rows. `open_table` checks that a table exists; `list_tables` returns names in the selected catalog; `drop_table` removes one named table and refuses an absent name. Invalid paths, invalid identifiers, empty schemas, duplicate columns, duplicate tables, and missing tables have named `LanceNamespaceError` variants. DuckDB operation errors propagate with their operation name. Table operations do not emulate `IF EXISTS`, `IF NOT EXISTS`, or `OR REPLACE`.

Paths are SQL string literals with apostrophes escaped. Catalog, table, and column names must start with an ASCII letter or underscore, followed by ASCII letters, digits, or underscores. They are still quoted as SQL identifiers. Other characters are refused before SQL because the Lance extension can create punctuation-named datasets that it does not enumerate through `SHOW TABLES`. A name containing SQL punctuation cannot execute another statement or leave an inaccessible dataset. The test uses a directory with an apostrophe and a table name containing an attempted `DROP TABLE` statement, then checks the original row is intact.

CI runs these tests with the pinned Linux amd64 extension in `bash scripts/check-lancedb-smoke.sh`. On an arm64 Docker host, `bash scripts/check-lancedb-smoke-docker.sh run` uses the official DuckDB v1.5.5 Linux arm64 artifact with SHA-256 `9592a76d4b24bc1cdd801afe436bc76998f6b99184162153edbb77431f2b5e56`; production startup verifies DuckDB `v1.5.5` and Lance build `2f167ea` before any table test.

## Tests

| Lane | Binary | Test |
|---|---|---|
| lancedb-smoke | lancedb_namespace_test | fresh_connection_reopens_persisted_table_under_unqualified_name |
| lancedb-smoke | lancedb_namespace_test | duplicate_create_and_missing_table_errors_preserve_existing_rows |
| lancedb-smoke | lancedb_namespace_test | drop_removes_only_selected_table_after_reopen |
| lancedb-smoke | lancedb_namespace_test | path_and_identifier_inputs_cannot_execute_extra_sql |
| lancedb-smoke | lancedb_namespace_test | invalid_column_schemas_fail_before_creating_a_dataset |
