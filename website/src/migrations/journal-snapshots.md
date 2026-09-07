# Journal and snapshots

Three files live next to each migration, and each answers a different question.

```text
migrations/
├── _journal.json                # what ran, in what order, with which hash
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

The journal decides the next migration number and, more importantly, pins each
file's content. `run_migrate` recomputes the SHA-256 of every pending file and
compares it with the entry:

```text
MigrateError::HashMismatch { file, expected, actual }
```

Editing a migration that already shipped therefore stops the run instead of
letting two environments drift apart silently. The fix is a new migration, not an
edit — the same rule every migration tool has, enforced here rather than
documented.

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
      "column_order": ["id", "email"],
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

`run_generate` loads the newest snapshot it can find, diffs it against the
current registry, and writes both the SQL and the next snapshot. `prev_id` links
a snapshot to the one it was diffed from.

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
    // e.g. ("users", "accounts") when both appear
    …
  }

  fn resolve_columns(&self, table: &str, added: &[String], removed: &[String]) -> Vec<(String, String)> {
    …
  }
}

let ops = diff_with_resolver(&old_snapshot, &registry, &MyRenames);
```

Resolved pairs become `RenameTable` / `RenameColumn` operations, which render as
`ALTER TABLE … RENAME` — data preserved instead of dropped.

`run_generate` uses the plain `diff` (the `NoRenames` resolver). To generate a
rename, call `diff_with_resolver` and `generate_sql_for` yourself, or write the
`ALTER … RENAME` statement into the migration by hand and keep the snapshot in
step.
