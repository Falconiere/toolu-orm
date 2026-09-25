# Session-selected query dialect

The same typed SELECT builder renders with `?N` for SQLite and Lance, and `$N` for PostgreSQL. The bound values stay in SQL order, including limits and offsets. Double quotes inside identifiers are doubled in table names, columns, and aliases. An offset without a limit gets SQLite's `LIMIT -1` prefix only for SQLite; Lance accepts `OFFSET ?1` directly.

`to_sql_for(dialect)` and the SELECT `to_count_sql_for`, `to_exists_sql_for`, and `to_first_row_sql_for` methods take an explicit dialect. `to_sql()` remains a compile-time `Dialect::CURRENT` shorthand. Existing builder execution methods now ask their executor for `dialect()` and render through the explicit form. Built-in libsql/rusqlite executors return SQLite; PostgreSQL clients and `PgTransaction` return PostgreSQL. A third-party executor can override the compatibility default when its runtime backend differs from `CURRENT`.

The mixed `postgres,lancedb` feature case is a rendering case: `CURRENT` is PostgreSQL, while an explicit Lance render still emits `?N`. This slice does not add a production Lance executor. The [pinned Lance SQL matrix](lancedb-sql-matrix.md) executes the Lance-rendered SELECT and plain DML against real disposable Lance tables. Its `ON CONFLICT` probe fails because the fixture has no unique key; ordinary DML `RETURNING` also fails. Issue #174 owns capability refusal before execution. The renderer keeps those clauses visible so they are never silently changed into plain writes.

Run the focused checks with:

```sh
cargo nextest run -p toolu-orm-query --test session_dialect_sql_test
cargo nextest run -p toolu-orm-query --features postgres,lancedb --test session_dialect_sql_test
cargo nextest run -p toolu-orm-query --features rusqlite --test session_executor_dialect_test
bash scripts/check-lancedb-smoke.sh
```

## Tests

| Lane | Binary | Test |
|---|---|---|
| default | lance_dialect_test | lance_parameters_and_identifiers_have_duckdb_syntax |
| default | lance_dialect_test | engine_specific_query_constructors_refuse_lance |
| default | lance_dialect_test | lance_migration_generation_stays_explicitly_unsupported |
| default | session_dialect_sql_test | one_builder_renders_for_three_selected_dialects |
| default | session_dialect_sql_test | lance_offset_without_limit_does_not_use_sqlite_workaround |
| default | session_dialect_sql_test | lance_writes_keep_bind_order_and_unsupported_clauses_explicit |
| default | session_dialect_sql_test | explicit_rendering_and_legacy_current_remain_distinct |
| default | session_dialect_sql_test | identifier_quotes_are_escaped_in_each_builder |
| rusqlite-only | session_executor_dialect_test | execution_uses_runtime_dialect_even_when_current_is_sqlite |
| rusqlite-only | session_executor_dialect_test | built_in_sqlite_executors_report_sqlite |
