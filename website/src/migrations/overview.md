# The migration loop

Migrations are generated from the diff between the last snapshot and your current
`SchemaRegistry`, written as plain SQL, and applied by name order with an
integrity check. Three functions cover the whole cycle.

```rust
use toolu_orm_cli::{generate::run_generate, migrate::run_migrate, status::get_status};
```

## Generate

```rust
let wrote: Option<String> =
  run_generate(&registry, "migrations", "add_posts", Dialect::Postgres)?;
// Some("0002_add_posts.sql"), or None when the schema did not change
```

`run_generate` reads `_journal.json`, loads the snapshot of the newest entry,
diffs it against the registry, and — only if there is a difference — writes three
things: the numbered `.sql` file, its `.snapshot.json`, and a new journal entry
holding the file's SHA-256.

The file number is `max(existing) + 1`, zero-padded to four digits. The dialect
argument decides the SQL that is written, so a project targeting both databases
generates into two directories.

Generation never touches a database — it is pure file I/O over your structs, so
it is safe to run in a build script or a small `bin/`.

## Migrate

```rust
let applied: u32 = run_migrate(&conn, "migrations", Dialect::Postgres).await?;
```

`run_migrate` creates the `_migrations` bookkeeping table if needed, reads the
applied set from it, verifies each pending file's hash against the journal, and
runs the pending files in journal order — which is the order they were
generated, and therefore their number order. Each file executes inside its own
`BEGIN` / `COMMIT`: a statement that fails rolls that file back and stops the
run, leaving earlier files applied and recorded.

Files with several statements separate them with a `--> statement-breakpoint`
line:

```sql
CREATE TABLE "posts" (
  "id" TEXT PRIMARY KEY,
  "author_id" TEXT NOT NULL REFERENCES "users"("id") ON DELETE CASCADE
);
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "idx_posts_author" ON "posts" ("author_id");
```

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

Embedding does not weaken the integrity check, it strengthens it. On disk the
`.sql` can be edited after install; here the bytes are frozen at compile time,
and the shipping project can catch an un-re-hashed edit in its own test suite:

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
  introspected to verify it.

## Status

```rust
let status = get_status(&conn, "migrations", Dialect::Postgres).await?;
println!("applied: {:?}", status.applied);
println!("pending: {:?}", status.pending);
```

`MigrationStatus` is two `Vec<String>` of file names — what the database has
recorded, and what is on disk but not yet applied.

## What the diff can express

The diff produces a list of `Operation`s, which `generate_sql_for` renders per
dialect:

| Group | Operations |
|---|---|
| Tables | `CreateTable`, `DropTable`, `RenameTable` |
| Columns | `AddColumn`, `DropColumn`, `RenameColumn`, `AlterColumn` (type, default, nullability, uniqueness) |
| Indexes | `CreateIndex`, `DropIndex` |
| Constraints | `AddForeignKey`, `DropForeignKey`, `AddCheckConstraint`, `DropCheckConstraint` |
| Enums | `CreateEnum`, `AlterEnum`, `DropEnum` |

Because the output is a file, the review question is the usual one: read the SQL
in the pull request. Nothing is applied at generation time and nothing is
rewritten later.

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
