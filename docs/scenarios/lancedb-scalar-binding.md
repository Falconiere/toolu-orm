# LanceDB portable scalar binding

**Scope:** Issue #148 converts existing `toolu_orm_core::value::Value` inputs to prepared DuckDB parameters for the local Lance session. Result decoding and portable query execution are separate work.

`toolu_orm::to_duckdb_params(&values)?` (or `toolu_orm_connection::to_duckdb_params`) converts the entire slice to owned `duckdb::types::Value` entries. Pass `duckdb::params_from_iter(params.iter())` to a prepared DuckDB statement. Convert before executing: an unsupported tagged value returns `LanceValueError::Unsupported { kind }`, and no statement has run. The conversion never renders a value into SQL text.

| Portable `Value` | Bound DuckDB type |
|---|---|
| `Null` | SQL `NULL` |
| `Integer(i64)` | `BIGINT` |
| `Real(f64)` | `DOUBLE` |
| `Text(String)` | `VARCHAR` |
| `Blob(Vec<u8>)` | `BLOB` |
| `Boolean(bool)` | `BOOLEAN` |

`TimestampEpoch`, `TimestampText`, `Json`, `Uuid`, and `Numeric` return typed conversion errors. Their codecs are outside epic #145. DuckDB still reports its own prepare, placeholder-count, destination-type, and execution errors for supported scalar values.

The real test opens the pinned Lance extension, creates a temporary local dataset, and inserts all six scalar forms through one prepared statement. It reads back the stored types, then matches quoted text, a binary payload, and NULL through bound predicates. A second prepared insert is never executed when any deferred variant appears after a valid parameter; the seeded row remains unchanged.

Run the pinned extension and production test lane with `bash scripts/check-lancedb-smoke.sh`.

## Tests

| Lane | Binary | Test |
|---|---|---|
| lancedb-smoke | lancedb_value_test | prepared_insert_stores_each_portable_scalar_type |
| lancedb-smoke | lancedb_value_test | prepared_filters_treat_quoted_text_null_and_binary_as_data |
| lancedb-smoke | lancedb_value_test | deferred_values_fail_conversion_before_a_prepared_write |
