# toolu-orm-cli

Migration tooling for toolu-orm — generates, applies, and checks migration status.

## Crate Type
- Library
- Internal deps: toolu-orm-core

## Crate-Specific Rules
- Migrations are split on `"--> statement-breakpoint"` separator for multi-statement execution
- Journal (`_journal.json`) tracks migration order and SHA256 hashes
- Snapshots (`.snapshot.json`) serialize schema state for diffing
- `run_migrate()` falls back to directory scanning if no journal exists
- Hash verification prevents applying tampered migrations

## Key Modules
- `generate.rs` — `run_generate()`: diff current schema vs latest snapshot, produce .sql migration
- `migrate.rs` — `run_migrate()`: apply pending migrations, track in `_migrations` table
- `status.rs` — `get_status()`: report applied vs pending migrations

## References
- Root CLAUDE.md (project-wide rules)
- `docs/rules/forbidden-syntax-rust.md`
