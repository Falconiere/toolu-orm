# Value round-trip

**Feature:** `Value { Null, Integer(i64), Real(f64), Text(String), Blob(Vec<u8>) }` is the one parameter type every builder and every driver accepts. Each driver converts it (`libsql::Value`, rusqlite `ToSql`, `to_pg_params`) and decodes it back through `FromRow`.
**Drivers:** libsql, rusqlite, Postgres.
**Spec:** AC-10.

## What is proven

Each variant is bound as a parameter into a table with matching column types and read back equal:

| Value | SQLite column | Postgres column | Read back |
|---|---|---|---|
| `Null` | any | `BIGINT` | `Option::None` |
| `Integer(i64::MAX)` | `INTEGER` | `BIGINT` | exact |
| `Real(1.5)` | `REAL` | `DOUBLE PRECISION` | exact |
| `Text("héllo")` | `TEXT` | `TEXT` | byte-for-byte UTF-8 |
| `Blob([0, 255, 7])` | `BLOB` | `BYTEA` | byte-exact |

On Postgres the values are also used as `WHERE` parameters (`id = $1 AND t = $2 AND r = $3 AND b = $4 AND n IS NULL`) to prove the bound bytes match what the server stored, both through the orm-query `Executor` and the orm-connection `DbConnection`.

## How to run

```sh
cargo nextest run -p toolu-orm-query --features libsql -E 'test(/value_/)'
cargo nextest run -p toolu-orm-query --features rusqlite -E 'test(/value_/)'
docker compose -f docker-compose.test.yaml up -d --wait
TEST_DB_PORT=5434 cargo nextest run -p toolu-orm-core -p toolu-orm-macros -p toolu-orm-query -p toolu-orm-connection -p toolu-orm-cli --features postgres -E 'test(/value_variant/)'
```

## Tests

| Lane | Binary | Test |
|---|---|---|
| libsql-only | libsql_mutations_test | value_null_round_trips_as_none |
| libsql-only | libsql_mutations_test | value_integer_round_trips_i64_max |
| libsql-only | libsql_mutations_test | value_real_round_trips |
| libsql-only | libsql_mutations_test | value_text_round_trips_unicode |
| libsql-only | libsql_mutations_test | value_blob_round_trips_byte_exact |
| rusqlite-only | rusqlite_mutations_test | value_null_round_trips_as_none |
| rusqlite-only | rusqlite_mutations_test | value_integer_round_trips_i64_max |
| rusqlite-only | rusqlite_mutations_test | value_real_round_trips |
| rusqlite-only | rusqlite_mutations_test | value_text_round_trips_unicode |
| rusqlite-only | rusqlite_mutations_test | value_blob_round_trips_byte_exact |
| postgres | postgres_mutations_test | every_value_variant_binds_and_reads_back |
| postgres | postgres_live_queries_test | query_map_binds_every_value_variant |
