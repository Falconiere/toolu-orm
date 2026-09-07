# Error handling

Each layer has its own error type, and each one converts into the next. Nothing
panics: `unwrap`, `expect`, `panic!`, `unreachable!` and slice indexing are
denied by clippy across `src/`, so a failure is always a `Result` you can match.

## `DbCoreError` — schema and rows

`toolu_orm_core::error::DbCoreError`

| Variant | Raised when |
|---|---|
| `SnapshotRead` / `SnapshotWrite` | A snapshot file cannot be read or written. |
| `MigrationWrite` | The generated `.sql` file cannot be written. |
| `JournalRead` / `JournalWrite` | `_journal.json` is unreadable, malformed, or unwritable. |
| `RowMapping` | A column cannot be read or converted into the target field. |
| `Connection` | Connection initialisation failed at the core layer. |

## `QueryError` — builders and executor

`toolu_orm_query::QueryError`

| Variant | Raised when |
|---|---|
| `Driver(..)` | The driver rejected the statement. The inner type is the active driver's error. |
| `RowMapping { field, source }` | A row could not be decoded; `field` names the column. |
| `NotFound { table }` | `fetch_one` found no row. |
| `Transaction(..)` | A transaction failed, wrapping the cause. |

`fetch_optional` returns `Ok(None)` where `fetch_one` returns
`Err(QueryError::NotFound)` — pick the one that matches whether an absent row is
an error in your call site.

## `DbError` — connections

`toolu_orm_connection::DbError`

| Variant | Raised when |
|---|---|
| `Connection` | Opening a database or replica failed. |
| `Query` | A statement failed. On Postgres the message carries severity, message and SQLSTATE. |
| `Transaction` | Begin, commit or rollback failed. |
| `Pool` | The pool could not hand out a connection. |
| `RowMapping` | Row decoding failed (converted from `DbCoreError`). |

## `MigrateError` — the migration runner

`toolu_orm_cli::migrate::MigrateError`

| Variant | Raised when |
|---|---|
| `Database(..)` | A statement in a migration failed. That file is rolled back. |
| `ReadDir(..)` | The migrations directory cannot be read. |
| `ReadFile(..)` | A migration file cannot be read, or the journal is malformed. |
| `HashMismatch { file, expected, actual }` | A migration changed after it was recorded. |

`HashMismatch` is the one worth handling explicitly in a deployment path: it
means the code and the database disagree about history, and continuing would
apply a file nobody reviewed.

```rust
match run_migrate(&conn, "migrations", Dialect::Postgres).await {
  Ok(n) => println!("applied {n}"),
  Err(MigrateError::HashMismatch { file, .. }) => {
    return Err(format!("migration {file} was edited after it shipped").into());
  },
  Err(e) => return Err(e.into()),
}
```

All four types implement `std::error::Error` through `thiserror`, so `?` into
`Box<dyn Error>`, `anyhow`, or your own enum works as usual.
