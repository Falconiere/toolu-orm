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
