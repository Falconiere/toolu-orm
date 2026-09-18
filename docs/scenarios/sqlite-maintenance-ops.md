# SQLite maintenance operations

**Feature:** Typed SQLite administration on a **borrowed** `&rusqlite::Connection`
— `VACUUM INTO`, `PRAGMA quick_check`, `ATTACH`/`DETACH` with cleanup guaranteed
by `Drop`, and typed `page_count` / `page_size` reads. Lives in
`toolu-orm-connection` as `SqliteMaintenance` and `AttachedDatabase`
(`crates/orm-connection/src/rusqlite_impl/maintenance/`), gated on the
`rusqlite` feature alone.
**Drivers:** rusqlite only. `VACUUM INTO` and `ATTACH` are SQLite statements,
and libsql's embedded-replica model gives attaching a second file different
semantics.
**Spec:** issue #115, `docs/toolu/specs/2026-09-18-115-sqlite-maintenance-ops-design.md`,
AC-1 through AC-14.

## Why it is not a builder

`VACUUM`, `ATTACH`, `DETACH` and `PRAGMA` are control statements: they name
schemas and files, not tables and columns, so no `SelectBuilder` can reach them.
Consumers were hand-building them as strings, which is where the two real bugs
this page closes come from — an attachment identifier interpolated without
quoting, and a `DETACH` that is skipped on the error path.

The surface borrows. It never opens a database, never begins a transaction,
never takes ownership; the caller keeps its connection and its transactions, and
gets its connection back in the state its own statements left it in.

## Which crate owns it, and why

`orm-connection`, not `orm-query`. `orm-query` compiles its executor tree inside
`cfg_single_backend!`, which needs *exactly one* driver feature — a gate that
exists because `QueryError::Driver` and `FromRow`'s shape change per driver set.
Maintenance operations touch neither, so inheriting that restriction would mean a
consumer whose graph unifies `rusqlite` with `postgres` silently loses the API.
`#[cfg(feature = "rusqlite")] pub mod rusqlite_impl` has no such gate. The cost:
a consumer naming only `toolu-orm-query` imports from `toolu_orm_connection`
instead — which `toolu-orm-query/rusqlite` already depends on, and which the
facade re-exports as `toolu_orm::connection`.

## The surface

```rust
use toolu_orm_connection::SqliteMaintenance;

// A validated pre-upgrade snapshot, end to end.
conn.vacuum_into(Path::new("/var/app/snapshot.db"))?;
let snapshot = conn.attach_database(Path::new("/var/app/snapshot.db"), "snapshot")?;
let report = snapshot.quick_check()?;
snapshot.detach()?;
if !report.is_ok() {
  return Err(format!("snapshot is unusable: {report}").into());
}

// A copy between two databases; the detach happens on every exit path.
let old = conn.attach_database(&archive, "old")?;
conn.execute("INSERT INTO t SELECT * FROM old.t", [])?;  // a failure here still detaches
old.detach()?;

// Storage statistics.
let stats = conn.storage_stats()?;
println!("{} bytes over {} pages", stats.bytes(), stats.page_count);
```

Design points the tests pin:

- **The filename is always a bound parameter.** SQLite documents the filename of
  both `VACUUM … INTO <expr>` and `ATTACH DATABASE <expr> AS …` as an
  *expression*, so `?1` is legal in both and no path can alter a statement.
- **The schema identifier is the only thing rendered into SQL**, and it is
  rendered in exactly one place (`schema_name.rs`): always double-quoted, with
  every interior `"` doubled. That is total, not best-effort: inside a SQLite
  double-quoted identifier the *only* escape is `""` — there is no backslash
  escape — so once every interior quote is doubled, the sole unpaired quotes are
  the delimiters, and `;`, a newline, `--` and `/* */` are all ordinary
  characters of the name. `an_injection_shaped_schema_name_is_quoted_not_executed`
  proves it on both statements rather than arguing it. Schema-qualified `PRAGMA`
  reads go through rusqlite's own `pragma_query` / `pragma_query_value`, which
  apply the same rule.
- **Every read names its schema.** A schema-less `PRAGMA quick_check` checks
  *all* attached databases, so `conn.quick_check()` issues `PRAGMA main.quick_check`
  and means the main database whatever is attached; `AttachedDatabase::quick_check`
  is its exact counterpart for one attachment.
- **`Drop` detaches, `detach()` reports.** The guard borrows the connection, so
  it cannot outlive it; it issues `DETACH` on every exit path, including the `?`
  that abandons a half-finished copy. `Drop` has nowhere to return a failure, so
  it logs at `warn`; `detach()` runs the same statement and hands back its
  result. Either way `DETACH` is issued once.
- **SQLite's errors are preserved, not stringified.** `MaintenanceError::Sqlite`
  holds a `rusqlite::Error`, so its result code survives. Only two refusals are
  this API's own — an empty schema name and one holding a NUL byte — and both
  happen before any statement is sent.
- **Maintenance statements skip the prepared-statement cache**
  ([Prepared-statement cache](prepared-statement-cache.md)): they are one-shot
  administrative calls, and caching one would evict a hot query for no gain.

## Supported surface and the FFI exception

Registering a custom FTS5 tokenizer — `SELECT fts5(?1)` bound with
`sqlite3_bind_pointer(.., "fts5_api_ptr", ..)`, then a C function pointer out of
`fts5_api` — is **deliberately outside** this ORM's supported surface, and
`RusqliteConnection::with_raw_connection` is the sanctioned escape hatch. Three
reasons:

1. A host pointer is not a value of any SQL type. `Value` and rusqlite's `ToSql`
   model SQL *data*, so this is not a gap in `Value` that could be filled.
2. Supporting it means re-exporting `libsqlite3-sys` types and owning `unsafe` in
   `orm-connection`. `[workspace.lints.rust] unsafe_code = "deny"` is workspace
   wide, and the one exception — the separately published
   `toolu-orm-sqlite-vec-register` — exists precisely so a single `unsafe`
   registration call had somewhere to live. A second one is a policy change, not
   a feature.
3. A tokenizer belongs to a *connection*, not to a schema, a table, or a query.
   That is connection bootstrap, which this ORM already delegates —
   `from_connection` exists so pragmas, open flags and loaded extensions are
   configured by the caller and adopted as-is.

Register on your own `rusqlite::Connection`, either before `from_connection` or
afterwards through `with_raw_connection`. Everything downstream of registration
stays fully supported: the `#[fts5_table]` schema, `MATCH`, `bm25`, `snippet`,
`highlight` ([FTS5 queries](fts5-queries.md)) and `vec0` KNN
([vec0 KNN](vec0-knn.md)). Only the registration handshake is out of scope.

## What is proven

Every test runs against a real database file in a real temporary directory
(`crates/orm-connection/tests/fixtures/temp_db_dir.rs`, removed on `Drop`, no
`tempfile` dependency). `VACUUM INTO` and `ATTACH` name files, so an in-memory
database cannot stand in for the database under test.

| Scenario | Real input | Result |
|---|---|---|
| A snapshot is a real database | `src.db` with 3 rows | the destination file exists and a fresh connection reads back the same 3 rows |
| A path holding `'` and `"` | a temp directory named `qu'ote"dir`, files `sr'c".db` / `sn'ap".db` | vacuum and attach both succeed; the attached snapshot holds the source's rows |
| The destination is taken | a real database already holding 3 different rows | `MaintenanceError::Sqlite(rusqlite::Error::SqliteFailure(..))` saying `output file already exists`; the occupant keeps its rows and the file is unchanged byte for byte |
| A validated snapshot | vacuum, attach, check, detach | `is_ok()`, `messages() == ["ok"]`, `Display` is `ok`, and nothing is left attached |
| An identifier holding `"` | schema `we"ird` over an empty main database | `database_list` shows `we"ird`, `schema()` returns it unchanged, `SELECT count(*) FROM "we""ird".t` is 2, and the guard detaches at end of scope |
| An identifier shaped like an injection | schemas `x"; DROP TABLE t; --`, `y"\n/* */; DELETE FROM t; --`, `""; ATTACH DATABASE ':memory:' AS pwned; --` | each lands as one identifier (`database_list` shows the whole string, `schema()` returns it unchanged) and the victim table keeps its 3 rows — through `ATTACH`, through the guard's `Drop`, and through `detach()`, which runs on `execute_batch` and would really execute a second statement if one escaped |
| A copy that fails | `INSERT … SELECT … FROM old.does_not_exist` behind a `?` | SQLite's `no such table` survives the early return; `database_list` is back to `main`; the same name attaches again |
| Explicit detach | `detach()` twice over, then a schema detached behind the guard's back | `Ok(())` and a clean `database_list` in the first case; `no such database` preserved as `SqliteFailure` in the second |
| A name this API refuses | `""` and `"a\0b"` | `InvalidSchemaName` naming `it is empty` / `it contains a NUL byte`; `database_list` shows nothing was attached |
| A file that is not a database | a file holding one line of plain text, no SQLite header | `SqliteFailure` saying `file is not a database`; no guard, nothing attached |
| A real integrity problem | a row with two `NULL`s under a schema rewritten to `NOT NULL` via `PRAGMA writable_schema`, on a connection closed before the file is attached | `!is_ok()`, `messages()` is exactly `["NULL value in q.b", "NULL value in q.c"]` and `Display` joins them; it is a report, not an error; the reporting connection's own `quick_check()` is still `ok` |
| Page numbers are real | 200 rows at `page_size = 4096`, default journal mode | `page_size()` is 4096, `bytes() == page_count * 4096`, and `bytes()` equals the file's length on disk; 400 more rows raise `page_count`; an attachment for an absent file reads `page_count() == 0` |
| The main-database reads | a seeded database, through the trait rather than a guard | `storage_stats()` agrees with `page_count()` / `page_size()`, `bytes()` multiplies out, `quick_check()` is `ok` |
| A path that is not UTF-8 | `OsStr::from_bytes(b"\xff\xfe-not-utf8.db")` (unix) | `NonUtf8Path` from both entry points, no file created, nothing attached |
| The escape hatch | `RusqliteConnection::from_connection` over a file database | `with_raw_connection` reads the same `storage_stats` the raw connection reported; a `CREATE TABLE` issued inside the closure is written to by `DbConnectionBlocking::execute_sql`, and that write is what the next closure reads back |

## Limitations

- **`VACUUM` and a transaction do not mix.** SQLite answers `cannot VACUUM from
  within a transaction`; take the snapshot before you begin.
- **`DETACH` fails while your transaction still holds the attachment**
  (`database "<schema>" is locked`). Commit or roll back before the guard goes
  out of scope, or call `detach()` so the failure is reported rather than logged.
- **A `Drop`-time detach failure is a `tracing::warn!`, not an error.** `Drop`
  has nowhere to return a `Result`, and panicking there would be worse.
- **`detach()` does not retry.** The guard is consumed either way, so a reported
  failure is final rather than silently attempted again.
- **An attach path that does not exist is created as an empty database.** That is
  SQLite's documented behavior for a writable connection, not something this API
  adds.
- **An empty schema name is refused although SQLite accepts one.** It really does
  address a distinct database, but an empty identifier is far more often an unset
  configuration value than an intent.
- **No async twin.** These are file-level operations on a borrowed sync
  connection; a caller that wants them off-thread already owns that choice.

## How to run

```sh
cargo nextest run -p toolu-orm-connection --features rusqlite -E 'binary(rusqlite_maintenance_test)'
```

## Tests

| Lane | Binary | Test |
|---|---|---|
| rusqlite-only | rusqlite_maintenance_test | snapshot::vacuum_into_snapshots_a_real_database |
| rusqlite-only | rusqlite_maintenance_test | snapshot::vacuum_into_refuses_an_existing_destination |
| rusqlite-only | rusqlite_maintenance_test | snapshot::a_vacuum_snapshot_passes_quick_check_through_an_attachment |
| rusqlite-only | rusqlite_maintenance_test | snapshot::vacuum_into_and_attach_bind_a_path_containing_quotes |
| rusqlite-only | rusqlite_maintenance_test | attach::attach_quotes_a_schema_name_containing_a_quote |
| rusqlite-only | rusqlite_maintenance_test | attach::an_injection_shaped_schema_name_is_quoted_not_executed |
| rusqlite-only | rusqlite_maintenance_test | attach::a_failed_copy_still_detaches_the_attached_database |
| rusqlite-only | rusqlite_maintenance_test | attach::explicit_detach_clears_the_attachment_and_reports_its_own_failure |
| rusqlite-only | rusqlite_maintenance_test | attach::attach_rejects_an_unusable_schema_name |
| rusqlite-only | rusqlite_maintenance_test | attach::attaching_a_file_that_is_not_a_database_preserves_the_sqlite_error |
| rusqlite-only | rusqlite_maintenance_test | inspect::quick_check_reports_a_real_integrity_problem |
| rusqlite-only | rusqlite_maintenance_test | inspect::attached_page_counts_match_the_file_on_disk |
| rusqlite-only | rusqlite_maintenance_test | inspect::main_database_reads_go_through_the_same_pragmas |
| rusqlite-only | rusqlite_maintenance_test | raw_access::a_non_utf8_path_is_refused_before_the_driver |
| rusqlite-only | rusqlite_maintenance_test | raw_access::with_raw_connection_borrows_the_driver_connection |
