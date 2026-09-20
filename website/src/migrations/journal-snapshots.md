# Journal and snapshots

Each migration has SQL and a snapshot; the directory shares one journal.

```text
migrations/
├── _journal.json                # declared order and expected content hashes
├── 0001_init.sql                # the statements
├── 0001_init.snapshot.json      # the schema state after this file
├── 0002_add_posts.sql
└── 0002_add_posts.snapshot.json
```

## The journal

```json
{
  "version": 1,
  "entries": [
    { "idx": 0, "name": "0001_init.sql", "hash": "sha256:8f14e45…", "created_at": 1717171717 }
  ]
}
```

The journal declares migration order and expected hashes. It does not record
what a particular database has run — the database's `_migrations` table does
that. It also determines the next number from the largest numeric filename
prefix in its entries.

`run_migrate` first checks already-applied entries against `_migrations`, then
checks each pending file as it reaches it. Two errors distinguish the failures:

```text
MigrateError::HashMismatch { file, expected, actual }
MigrateError::HistoryMismatch { file, recorded, declared }
```

`HashMismatch` means the SQL bytes differ from the declared hash.
`HistoryMismatch` means an applied migration's declared hash differs from the
hash recorded when it ran. Restore the shipped file or declaration, and express
new changes in a new migration. Updating both a shipped file and its journal
hash still fails against an existing database.

Validation has explicit limits: records with an empty hash are skipped without
verification, and records absent from the current journal are not checked.
An already-applied SQL file may be pruned; its recorded and declared hashes are
still compared, but its missing bytes cannot be verified. Other file-read
failures remain errors. With no journal entries, the runner uses the legacy
directory scan and records empty hashes.

## Snapshots

A snapshot is the schema as JSON after its migration:

```json
{
  "version": 1,
  "dialect": "postgres",
  "id": "…",
  "prev_id": "…",
  "tables": {
    "users": {
      "column_order": ["id"],
      "columns": { "id": { "name": "id", "column_type": "Text", "primary_key": true, "not_null": true, "unique": false } },
      "indexes": {},
      "foreign_keys": {},
      "check_constraints": {},
      "strict": false
    }
  },
  "enums": {},
  "meta": { "tables": {}, "columns": {} }
}
```

`run_generate` walks journal entries backwards and loads the first associated
snapshot file it finds. An empty journal starts from an empty schema; a
nonempty journal with no surviving snapshot returns `DbCoreError::SnapshotRead`.
It diffs against the current registry and writes both SQL and the next snapshot
when the schema changes. `prev_id` normally links to the snapshot used for the
diff; when that snapshot contains no tables, it is the zero UUID.

Snapshots are the review surface for schema changes: the SQL says what runs, the
snapshot diff says what the schema becomes. Both belong in the pull request.

Older snapshot layouts still deserialize — fields added later have serde
defaults, so a migration folder from an earlier version keeps diffing.

## Renames

By default the diff reports a rename as a drop plus a create, because two names
in a JSON file carry no evidence that one became the other. A `RenameResolver`
supplies that evidence:

```rust
use toolu_orm_core::diff::diff_with_resolver;
use toolu_orm_core::rename::RenameResolver;

struct MyRenames;

impl RenameResolver for MyRenames {
  fn resolve_tables(&self, added: &[String], removed: &[String]) -> Vec<(String, String)> {
    if removed.iter().any(|name| name == "users")
      && added.iter().any(|name| name == "accounts")
    {
      vec![("users".into(), "accounts".into())] // (old, new)
    } else {
      Vec::new()
    }
  }

  fn resolve_columns(&self, _table: &str, _added: &[String], _removed: &[String]) -> Vec<(String, String)> {
    Vec::new()
  }
}

let ops = diff_with_resolver(&old_snapshot, &registry, &MyRenames)?;
```

Resolved pairs become `RenameTable` / `RenameColumn` operations, which render as
`ALTER TABLE … RENAME` — data preserved instead of dropped.

`run_generate` uses the plain `diff` (the `NoRenames` resolver). To generate a
rename, call `diff_with_resolver` and `generate_sql_for` yourself, or write the
`ALTER … RENAME` statement into a new, unapplied migration. Keep its snapshot and
journal hash in step; `toolu_orm_core::journal::compute_hash` computes the expected
hash from the final SQL text. Do not edit an already-applied migration.
