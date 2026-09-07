//! The on-disk half of the baseline fixture: a migrations directory plus the
//! `_journal.json` describing it, shared by `migrate_baseline_test.rs` and
//! `migrate_baseline_postgres_test.rs`.
//!
//! `INIT_SQL` is written so that re-executing it on an adopted database fails
//! (`users` is already there) and so that one table, `audit`, exists only if a
//! statement actually ran — that is what proves a baseline executes nothing.

use toolu_orm_core::journal::{compute_hash, Journal};

pub const INIT_SQL: &str = "CREATE TABLE users (id TEXT PRIMARY KEY);\n\
   --> statement-breakpoint\nCREATE TABLE audit (id TEXT);";
pub const POSTS_SQL: &str = "CREATE TABLE posts (id TEXT PRIMARY KEY);";
pub const THIRD_SQL: &str = "CREATE TABLE c (id TEXT);";

type FixtureResult<T> = Result<T, Box<dyn std::error::Error>>;

/// A fresh `migrations` directory; the returned `TempDir` must stay alive.
///
/// # Errors
///
/// Returns the tempdir, create-dir, or non-UTF8 path error.
pub fn migrations_dir() -> FixtureResult<(tempfile::TempDir, String)> {
  let dir = tempfile::tempdir()?;
  let path = dir.path().join("migrations");
  std::fs::create_dir_all(&path)?;
  Ok((dir, path.to_str().ok_or("non-UTF8 path")?.to_owned()))
}

/// Writes each `(name, sql)` file and a journal listing them in that order.
///
/// # Errors
///
/// Returns the file-write or journal-write error.
pub fn write_migrations(dir: &str, files: &[(&str, &str)]) -> FixtureResult<()> {
  let mut journal = Journal::empty();
  for (name, sql) in files {
    std::fs::write(format!("{dir}/{name}"), sql)?;
    journal.add_entry(name, &compute_hash(sql));
  }
  journal.write_to_path(&format!("{dir}/_journal.json"))?;
  Ok(())
}
