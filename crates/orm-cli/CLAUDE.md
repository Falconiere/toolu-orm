# toolu-orm-cli

Migration tooling for toolu-orm — generates, applies, and checks migration status.

## Crate Type
- Library
- Internal deps: toolu-orm-core, toolu-orm-connection
- Features: `libsql` (default), `rusqlite`, `postgres`; blocking migration/status APIs use rusqlite

## Crate-Specific Rules
- Migrations are split on `"--> statement-breakpoint"` separator for multi-statement execution
- Journal (`_journal.json`) tracks migration order and SHA256 hashes
- Snapshots (`.snapshot.json`) serialize schema state for diffing
- `run_migrate()` falls back to directory scanning if no journal exists
- Hash verification prevents applying tampered migrations, and every run re-checks already-applied history (`migrate/history.rs`) before skipping an entry

## Key Modules
- `generate.rs` — `run_generate()`: diff current schema vs latest snapshot, produce .sql migration
- `migrate/` — Directory and embedded runners, baseline APIs and their blocking twins; track applied migrations in `_migrations`
- `status.rs` — Directory/embedded, async/blocking status APIs

## References
- Root CLAUDE.md (project-wide rules)
- Root `Cargo.toml`, `clippy.toml` and `scripts/check-file-length.sh`
