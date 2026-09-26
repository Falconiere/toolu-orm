# LanceDB scalar result decoding

`LanceDbConnection::query_map` (blocking or async) materializes real DuckDB–Lance results into an owned `LanceRow`. Implement `FromRow::from_lance_row` manually and call `row.get_typed::<T>("column")?` for each field. The raw `row.get("column")?` remains available. Derive support is separate (#161).

| DuckDB result | Portable value | Rust field |
|---|---|---|
| Signed/unsigned integer | `Integer(i64)` | `i64` |
| FLOAT, DOUBLE | `Real(f64)` | `f64` |
| VARCHAR | `Text(String)` | `String` |
| BOOLEAN | `Boolean(bool)` | `bool` |
| BLOB | `Blob(Vec<u8>)` | `Vec<u8>` |
| NULL or a supported value | `Null` or corresponding value | `Option<T>` |

HUGEINT, UHUGEINT and UBIGINT are checked against the i64 range; overflow fails instead of wrapping. FLOAT widens exactly to f64. Rust scalar kinds are strict: integers are not converted to booleans, floating-point values or strings. Text and binary data are owned, including empty strings/blobs and embedded zero bytes. Non-finite floating-point values remain floating-point values.

`LanceRow::get_typed` uses the exported `core::row::FromLanceValue` trait. It reports `DbCoreError::RowMapping` containing the column name and expected Rust type for absent fields, required NULLs and mismatched types; the session preserves that as `DbError::RowMapping`. `Option<T>` accepts NULL but still rejects a missing column or wrong non-NULL type. Errors do not include the cell payload. Column lookup is ASCII case-insensitive; duplicate projected names on a returned row are rejected. Empty results return an empty vector without invoking the decoder or validating `REQUIRED_COLUMNS`.

Native decimal, temporal and nested values return a column-specific unsupported-result error. UUID/JSON/temporal/decimal tagged codecs remain outside epic #145; applications must explicitly project a supported scalar representation if desired. This decoder does not attach tagged semantics to ordinary text.

Run `bash scripts/check-lancedb-smoke.sh` for the pinned extension and production suites. Tests create temporary persisted Lance tables and query them through the shared connection traits.

## Tests

| Lane | Binary | Test |
|---|---|---|
| lancedb-smoke | lancedb_row_test | scalars::typed_scalars_round_trip_through_blocking_and_async_sessions |
| lancedb-smoke | lancedb_row_test | scalars::missing_null_and_mismatched_columns_name_expected_rust_type |
| lancedb-smoke | lancedb_row_test | boundaries::integer_widths_are_checked_and_aliases_are_case_insensitive |
| lancedb-smoke | lancedb_row_test | boundaries::unsupported_results_and_duplicate_names_fail_explicitly |
| lancedb-smoke | lancedb_row_test | optional_real::every_optional_scalar_accepts_null_and_present_values |
| lancedb-smoke | lancedb_row_test | optional_real::float_widens_and_nonfinite_doubles_remain_floats |
| lancedb-smoke | lancedb_row_test | optional_real::required_null_and_optional_mismatches_never_coerce |
