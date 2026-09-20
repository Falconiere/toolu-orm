# Error handling

Each layer reports errors through `Result`. Core errors convert into query or
connection errors; migration errors usually retain database failures as strings.
These conversions do not preserve every driver's structured error. A `Result`
API also does not catch panics in caller-provided row decoders.

## `DbCoreError` — schema and rows

`toolu_orm_core::error::DbCoreError`

| Variant | Raised when |
|---|---|
| `SnapshotRead` / `SnapshotWrite` | A snapshot file cannot be read or written. |
| `MigrationWrite` | The generated `.sql` file cannot be written. |
| `JournalRead` / `JournalWrite` | `_journal.json` is unreadable, malformed, or unwritable. |
| `RowMapping` | A column cannot be read or converted into the target field. |
| `Connection` | Connection initialisation failed at the core layer. |
| `VirtualTableChange` | A virtual-table schema change cannot be generated automatically. |
| `Fts5SyncInvalid` | An FTS5 synchronization declaration cannot produce valid triggers. |
| `Fts5UnsupportedDialect` / `Fts5InvalidArgument` | An FTS5 expression uses the wrong dialect or an invalid argument. |
| `InvalidVec0Identifier` / `Vec0BitDistanceMetric` | A vec0 declaration has an unsupported identifier or a distance metric on a bit vector. |
| `VectorDimension` | An embedding has the wrong number of elements. |
| `Vec0UnsupportedDialect` / `Vec0InvalidArgument` | A sqlite-vec expression uses the wrong dialect or an invalid argument. |
| `PgFtsUnsupportedDialect` / `PgFtsInvalidArgument` | A Postgres full-text expression uses the wrong dialect or an invalid argument. |
| `PgVectorUnsupportedDialect` / `PgVectorInvalidArgument` | A pgvector expression uses the wrong dialect or an invalid argument. |
| `InvalidScalarFunction` / `InvalidTableFunction` | A SQL function name fails identifier validation. |

`DbCoreError` is non-exhaustive: include a fallback arm when matching it.

## `QueryError` — builders and executor

`toolu_orm_query::QueryError`

| Variant | Raised when |
|---|---|
| `Driver(..)` | The driver rejected the statement. The inner type is the active driver's error. |
| `RowMapping { field, source }` | A row could not be decoded. Conversion from `DbCoreError` sets `field` to `"unknown"`; inspect `source` for decoding details. |
| `NotFound { table }` | `fetch_one` found no row. |
| `Transaction(..)` | A transaction failed, wrapping the cause. |
| `Connection(..)` | A rusqlite wrapper operation failed; contains the connection-layer error as text. Available only in the rusqlite-only query build. |

`Driver` is available only when exactly one driver feature is enabled on
`toolu-orm-query`. With rusqlite, execution through a raw `rusqlite::Connection`
preserves its driver error; execution through `RusqliteConnection` reports
`Connection` or `RowMapping` instead.

`fetch_optional` returns `Ok(None)` where `fetch_one` returns
`Err(QueryError::NotFound)` — pick the one that matches whether an absent row is
an error in your call site.

## `DbError` — connections

`toolu_orm_connection::DbError`

| Variant | Raised when |
|---|---|
| `Connection` | Opening a database or replica failed, or a rusqlite connection lock was poisoned. |
| `Query` | A statement failed. Postgres `execute_sql` and `query_map` include severity, message and SQLSTATE for server errors; `execute_batch` uses the driver's display text. |
| `Transaction` | Begin, commit or rollback failed. |
| `Pool` | The pool could not hand out a connection. |
| `RowMapping` | Row decoding failed (converted from `DbCoreError`). |

## `MigrateError` — the migration runner

`toolu_orm_cli::migrate::MigrateError`

| Variant | Raised when |
|---|---|
| `Database(..)` | A database operation failed, including bookkeeping, begin, commit, or rollback. A statement failure triggers a rollback attempt for that migration. |
| `ReadDir(..)` | The migrations directory cannot be read. |
| `ReadFile(..)` | A migration file cannot be read, or the journal is malformed. |
| `HashMismatch { file, expected, actual }` | SQL bytes differ from their declared hash, including when an already-applied file is checked again. |
| `HistoryMismatch { file, recorded, declared }` | An applied migration's source declares a different hash from the one stored in the database. |
| `NotInJournal(..)` | A baseline names a migration absent from its journal or embedded list. |
| `DuplicateMigration(..)` | An embedded migration list repeats a name. |
| `MissingExtension { file, module }` | A migration statement needs a SQLite module that is not registered on the connection. |
| `ForeignKeyViolation { file, count }` | A migration that suspended SQLite foreign keys leaves violations; it is rolled back. |
| `PragmaRestore { message, source }` | Restoring SQLite pragma settings failed; `source` retains an earlier migration failure when present. |

Both hash variants are worth handling in a deployment path. Restore the SQL
bytes for `HashMismatch` or the original declared hash for `HistoryMismatch`;
put new schema changes in a new migration. `MigrateError` is non-exhaustive, so
retain a fallback arm.

```rust
match run_migrate(&conn, "migrations", Dialect::Postgres).await {
  Ok(n) => println!("applied {n}"),
  Err(MigrateError::HashMismatch { file, .. }) => {
    return Err(format!("migration {file} does not match its declared hash").into());
  },
  Err(MigrateError::HistoryMismatch { file, .. }) => {
    return Err(format!("migration {file} disagrees with applied history").into());
  },
  Err(e) => return Err(e.into()),
}
```

## `MaintenanceError` — SQLite maintenance

`toolu_orm_connection::MaintenanceError` preserves a raw `rusqlite::Error` in
`Sqlite`, including result codes. `NonUtf8Path` and `InvalidSchemaName` report
validation failures before a statement runs. A completed integrity check with
problems returns an `IntegrityReport` with `is_ok() == false`, not an error.

These types implement `std::error::Error` through `thiserror`, so `?` works with
`Box<dyn Error>`, `anyhow`, or a custom enum with the appropriate `From` impls.
