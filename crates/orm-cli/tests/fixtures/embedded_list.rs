//! The in-memory half of the embedded-migration fixture, shared by
//! `migrate_embedded_test.rs` and `migrate_embedded_postgres_test.rs`.
//!
//! `EmbeddedMigration` borrows its three fields, which is what lets a consumer
//! write a `const` array of `include_str!`s. A test cannot do that, because its
//! hashes come from `compute_hash` at runtime, so [`OwnedMigration`] holds the
//! strings and [`list`] hands out the borrowed view.

use toolu_orm_cli::migrate::EmbeddedMigration;
use toolu_orm_core::journal::compute_hash;

/// Two statements split by the breakpoint separator, and a second table that
/// exists only if the second statement really ran.
pub const CREATE_SQL: &str = "CREATE TABLE users (id TEXT PRIMARY KEY);\n\
   --> statement-breakpoint\nCREATE TABLE audit (id TEXT);";
/// A migration whose second statement fails on both SQLite and Postgres.
pub const FAILING_SQL: &str =
  "CREATE TABLE half (id TEXT);\n--> statement-breakpoint\nINSERT INTO nope VALUES (1);";
/// Order-sensitive pair: the insert only works after the create.
pub const MAKE_T_SQL: &str = "CREATE TABLE t (id TEXT);";
pub const SEED_T_SQL: &str = "INSERT INTO t (id) VALUES ('seeded');";

/// A migration with owned strings, so its hash can be computed at runtime.
pub struct OwnedMigration {
  pub name: String,
  pub sql: String,
  pub hash: String,
}

/// A migration whose declared hash is the true hash of its SQL.
#[must_use]
pub fn honest(name: &str, sql: &str) -> OwnedMigration {
  OwnedMigration {
    name: name.to_owned(),
    sql: sql.to_owned(),
    hash: compute_hash(sql),
  }
}

/// A migration whose SQL was edited after its hash was declared — the shipped
/// `.sql` a consumer tampered with, or forgot to re-hash.
#[must_use]
pub fn tampered(name: &str, original: &str, edited: &str) -> OwnedMigration {
  OwnedMigration {
    name: name.to_owned(),
    sql: edited.to_owned(),
    hash: compute_hash(original),
  }
}

/// The borrowed view `run_migrate_embedded` takes.
#[must_use]
pub fn list(migrations: &[OwnedMigration]) -> Vec<EmbeddedMigration<'_>> {
  migrations
    .iter()
    .map(|m| EmbeddedMigration {
      name: &m.name,
      sql: &m.sql,
      hash: &m.hash,
    })
    .collect()
}

/// The `(name, sql)` pairs of the same migrations, for writing them to a
/// directory so the two sources can be compared.
#[must_use]
pub fn as_files(migrations: &[OwnedMigration]) -> Vec<(&str, &str)> {
  migrations
    .iter()
    .map(|m| (m.name.as_str(), m.sql.as_str()))
    .collect()
}
