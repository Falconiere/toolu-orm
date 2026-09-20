# The migration loop

Migrations are generated from the diff between the last snapshot and your current
`SchemaRegistry`, written as plain SQL, and applied in journal order with an
integrity check. Three functions cover the basic cycle. `toolu-orm-cli` is a
library; you wire these functions into your application's tooling.

```rust
use toolu_orm_cli::{generate::run_generate, migrate::run_migrate, status::get_status};
```

## Generate

```rust
let wrote: Option<String> =
  run_generate(&registry, "migrations", "add_posts", Dialect::Postgres)?;
// Some("0002_add_posts.sql"), or None when the schema did not change
```

Create the output directory before calling `run_generate`; it does not create
missing directories. It reads `_journal.json`, searches backwards through its
entries for the latest snapshot that still exists,
diffs it against the registry, and — only if there is a difference — writes three
things: the numbered `.sql` file, its `.snapshot.json`, and a new journal entry
holding the file's SHA-256.

The file number is one above the largest numeric filename prefix in the
journal (or 1 for an empty journal), padded to at least four digits. The dialect
argument decides the SQL that is written, so a project targeting both databases
generates into two directories.

Generation never touches a database — it is pure file I/O over your structs, so
it is safe to run in a build script or a small `bin/`.

## Migrate

```rust
let applied: u32 = run_migrate(&conn, "migrations", Dialect::Postgres).await?;
```

`run_migrate` creates the `_migrations` bookkeeping table if needed and validates
already-applied journal entries against the recorded hashes and surviving SQL
files. It then verifies each pending file's hash as it reaches it and runs it in
journal entry order, without sorting names. Each file executes inside its own
`BEGIN` / `COMMIT`: a statement that fails rolls that file back and stops the
run, leaving earlier files applied and recorded.

If the journal is missing or has no entries, the runner scans `.sql` files in
lexicographic filename order, skips recorded names, and executes each file as a
batch. This legacy path records an empty hash and provides no content integrity
check. A nonempty journal is authoritative: unlisted files do not run. See
[Journal and snapshots](journal-snapshots.md) for hash validation limits.

Generated files separate statements with a `--> statement-breakpoint` line.
Journaled and embedded migrations split on that marker; each chunk may also
contain multiple semicolon-separated statements:

```sql
CREATE TABLE "posts" (
  "id" TEXT PRIMARY KEY,
  "author_id" TEXT NOT NULL REFERENCES "users"("id") ON DELETE CASCADE
);
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "idx_posts_author" ON "posts" ("author_id");
```

The runner owns the transaction boundary. For SQLite rebuilds that request
`PRAGMA foreign_keys = OFF`, it suspends enforcement before `BEGIN` and restores
the caller's `foreign_keys` and `legacy_alter_table` settings afterwards. If
foreign keys were enabled initially, it also checks `foreign_key_check` before
commit and rolls back on violations.

## Ship the migrations inside the binary

`run_migrate` reads its SQL from disk, which a single-binary distribution does
not have: nothing installed by `cargo install`, a tap, or a `curl | sh` script
drops a `migrations/` directory next to the executable. Bake the SQL in with
`include_str!` instead:

```rust
use toolu_orm_cli::migrate::{run_migrate_embedded, EmbeddedMigration};

const MIGRATIONS: &[EmbeddedMigration] = &[
  EmbeddedMigration {
    name: "0001_init.sql",
    sql: include_str!("../migrations/0001_init.sql"),
    hash: "sha256:2c8f…",          // the same string _journal.json carries
  },
  // …
];

let applied: u32 = run_migrate_embedded(&conn, MIGRATIONS, Dialect::Sqlite).await?;
```

Everything below the byte-fetch is shared with `run_migrate`: the same
`_migrations` table, the same hash check, the same `--> statement-breakpoint`
splitting, and the same one transaction per migration. A database migrated from
a directory and one migrated from the equivalent list are indistinguishable, so
a project can switch sources between releases without re-running anything.

The SQL bytes are fixed at compile time. Applied entries are checked against
their recorded hashes before pending entries run, and the shipping project can
also check every declared hash in its own test suite:

```rust
#[test]
fn migrations_match_their_hashes() {
  for migration in MIGRATIONS {
    migration.verify_hash().expect("re-generate the hash");
  }
}
```

Two rules that differ from the on-disk path:

- Migrations apply in **list order**, not name order — the list is the
  declaration of order, exactly as `_journal.json` is on disk.
- A list naming the same migration twice is rejected with
  `MigrateError::DuplicateMigration` before anything runs. A generated journal
  cannot repeat a name; a hand-written array can.

## Baseline an existing database

A database that already carries the schema — built by a previous migration
system — must not have those migrations replayed against it. Record them as
applied instead:

```rust
use toolu_orm_cli::migrate::{mark_applied, mark_applied_through};

// "my database is already at 0016"
let recorded: u32 = mark_applied_through(&conn, "migrations", "0016_add_tags.sql", dialect).await?;

// or name them explicitly
let recorded = mark_applied(&conn, "migrations", &["0001_init.sql", "0002_add_posts.sql"], dialect).await?;
```

Both create `_migrations` if needed and insert one row per named journal entry,
carrying the hash from `_journal.json` — no SQL from those files is executed, and
the files themselves are never even read. The next `run_migrate` therefore starts
at the first entry you did not baseline, and a later edit to a still-pending file
is still caught by `MigrateError::HashMismatch`.

The rules worth knowing:

- A name with no journal entry is rejected with `MigrateError::NotInJournal`,
  which lists every unknown name and leaves the database untouched — silently
  recording an unknown name would throw away the hash check.
- Names already recorded are skipped, so a baseline is idempotent; the returned
  count is how many rows were newly written.
- The inserts share one transaction: a baseline either lands whole or not at all.
- Baselining asserts that the database really is at that version; nothing is
  introspected to verify it, and neither SQL bodies nor existing recorded hashes
  are validated by the baseline call. A later migration run validates history.

For an embedded list, use `mark_applied_embedded` or
`mark_applied_through_embedded` with the same arguments, replacing the directory
with `&[EmbeddedMigration]`. The list supplies order and declared hashes; the
baseline does not execute or verify its SQL.

## Status

```rust
let status = get_status(&conn, "migrations", Dialect::Postgres).await?;
println!("applied: {:?}", status.applied);
println!("pending: {:?}", status.pending);
```

`MigrationStatus` has two `Vec<String>` fields. `applied` follows database record
order; `pending` is every unrecorded `.sql` filename on disk, sorted by name.
Directory status scans files even when a journal exists, so it can list files
that a journaled migration run would ignore. It creates `_migrations` if needed
but does not check hashes or schema state.

`get_status_embedded(&conn, MIGRATIONS, dialect).await?` uses list order for
pending names and rejects duplicate names. It also reports names without
verifying SQL hashes.

## Without an async runtime

Every migration, baseline, and status API above has a blocking counterpart for
`&impl DbConnectionBlocking`. Append `_blocking` to the function name, including
after `_embedded`, and omit `.await`:

```rust
use toolu_orm_cli::{migrate::run_migrate_blocking, status::get_status_blocking};
use toolu_orm_core::rusqlite;
use toolu_orm_connection::RusqliteConnection;
use toolu_orm_core::dialect::Dialect;

let conn = RusqliteConnection::from_connection(rusqlite::Connection::open("app.db")?);
let applied = run_migrate_blocking(&conn, "migrations", Dialect::Sqlite)?;
let status = get_status_blocking(&conn, "migrations", Dialect::Sqlite)?;
```

For a rusqlite-only dependency on `toolu-orm-cli`, set `default-features = false`
and `features = ["rusqlite"]`; its default driver feature is `libsql`.

## What the diff can express

`diff` returns `Result<Vec<Operation>, DbCoreError>`, and `generate_sql_for`
renders successful operations per dialect:

| Group | Operations |
|---|---|
| Tables | `CreateTable`, `DropTable`, `RenameTable` |
| Columns | `AddColumn`, `DropColumn`, `RenameColumn`, `AlterColumn` (type, default, nullability, uniqueness, primary key, autoincrement, composite primary key) |
| Indexes | `CreateIndex`, `DropIndex` |
| Constraints | `AddForeignKey`, `DropForeignKey`, `AddCheckConstraint`, `DropCheckConstraint` |
| Enums | `CreateEnum`, `AlterEnum`, `DropEnum` |
| FTS5 | `RecreateFts5FromContent`, `CreateFts5SyncTriggers`, `DropFts5SyncTriggers` |

An operation does not guarantee executable SQL for both dialects. SQLite
column alterations trigger one create/copy/drop/rename rebuild per table,
including its declared indexes. Standalone foreign-key and CHECK changes render
comments requiring hand-written migration SQL. Postgres primary-key and
autoincrement alterations also render comments. SQLite virtual-table DDL is
commented out for Postgres. Inspect the generated file before applying it.

Unsupported virtual-table changes return `DbCoreError::VirtualTableChange`.
Eligible external-content FTS5 tables can be recreated and rebuilt from their
content table; other virtual-table changes need a hand-written migration.
The registry does not register named enum definitions; the enum operations are
a lower-level SQL-generation surface, not automatic generation from Rust enums.

## Wiring it into a binary

A small `bin/migrate.rs` is the common setup — the same registry the application
uses, one subcommand per function:

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let registry = app_schema();                    // your SchemaRegistry
  match std::env::args().nth(1).as_deref() {
    Some("generate") => {
      let name = std::env::args().nth(2).ok_or("usage: migrate generate <name>")?;
      match run_generate(&registry, "migrations", &name, Dialect::Sqlite)? {
        Some(file) => println!("wrote {file}"),
        None => println!("no schema change"),
      }
    },
    Some("migrate") => {
      let db = Database::init_local("data/app.db").await?;
      let conn = db.connect()?;
      println!("applied {}", run_migrate(&conn, "migrations", Dialect::Sqlite).await?);
    },
    _ => println!("usage: migrate [generate <name>|migrate]"),
  }
  Ok(())
}
```
