# LanceDB connection traits

**Scope:** Issue #186 adds `LanceDbConnection` for an attached local DuckDB–Lance catalog. Call `LanceDbConnection::from_namespace(LanceConnection::open(extension_path)?.attach(existing_directory, "data")?)` to transfer the selected connection into `DbConnection` or `DbConnectionBlocking`. Dropping the final session handle closes the DuckDB connection; a new open and attach reads persisted tables. The direct `LanceNamespace` API remains available before conversion.

The held DuckDB connection is `Send` but not `Sync`; one `std::sync::Mutex` serializes all SQL on it. Async callers wait for one permit before `spawn_blocking`, so contended calls do not occupy the blocking pool while waiting. The permit remains in the blocking task after the awaiting future is cancelled. Blocking calls take the mutex directly and need no Tokio runtime. A poisoned lock or failed blocking task returns `DbError::Connection`.

`execute_sql` and `query_map` convert all bound `Value` parameters before executing prepared SQL. Tagged timestamp, JSON, UUID, and numeric values fail with `LanceUnsupportedValue` inside `DbError::Query` before a write. `query_map` passes a `LanceRow` of named portable values to a manually implemented `FromRow::from_lance_row`; this slice decodes `BIGINT`, `VARCHAR`, and `NULL`. Missing or ambiguous column names, unsupported result types, and a row type without a Lance decoder return `DbError::RowMapping`. Full scalar codecs and derive support belong to #160 and #161.

`execute_batch` uses DuckDB's multi-statement execution. A supported `CREATE TABLE` script creates real Lance tables visible after reopen. An invalid or unsupported statement returns `DbError::Query` with the SQL and DuckDB diagnostic. A failed batch is not promised to roll back earlier statements; portable capability preflight belongs to #174.

## Tests

| Lane | Binary | Test |
|---|---|---|
| lancedb-smoke | lancedb_dbconnection_test | async_bound_write_and_typed_read_keep_sql_punctuation_as_data |
| lancedb-smoke | lancedb_dbconnection_test | batch_create_persists_after_reopen_and_reports_unsupported_ddl |
| lancedb-smoke | lancedb_dbconnection_test | blocking_methods_run_without_runtime_and_preserve_mapping_errors |
| lancedb-smoke | lancedb_dbconnection_test | concurrent_async_writes_serialize_on_one_duckdb_connection |
| lancedb-smoke | facade_only_lance_session_test | facade_only_lance_connection_methods_compile |
