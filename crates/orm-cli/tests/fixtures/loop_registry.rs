//! `#[table]` registries for the migration-loop tests: v1 has `users`; v2
//! adds `users.bio`, a unique index on `users.email`, and a `posts` table with
//! an FK to `users` and an enum-backed `status` column (TEXT + CHECK).
//!
//! `status` lives on the new table on purpose: SQLite cannot add a CHECK to an
//! existing table, and the generator currently emits only a comment for that
//! operation (see `docs/scenarios/migration-loop.md`).
//!
//! Wired into `migration_loop_{sqlite,postgres}_test.rs` with `#[path]`; every
//! item here is used by both binaries.

use toolu_orm_core::schema::SchemaRegistry;
use toolu_orm_core::table::TableSchema;
use toolu_orm_macros::ColumnEnum;

/// Stored as `TEXT` with `CHECK (status IN ('active', 'banned'))`.
#[derive(ColumnEnum)]
pub enum Status {
  Active,
  Banned,
}

pub mod v1 {
  use toolu_orm_core::column::{Integer, Text};
  use toolu_orm_macros::table;

  #[table(name = "users")]
  pub struct Users {
    #[column(primary_key)]
    pub id: Text,
    #[column(not_null)]
    pub name: Text,
    #[column(not_null)]
    pub email: Text,
    pub age: Integer,
  }
}

pub mod v2 {
  use toolu_orm_core::column::{Integer, Text};
  use toolu_orm_macros::table;

  use super::Status;

  #[table(name = "users")]
  #[unique_index("idx_users_email", email)]
  pub struct Users {
    #[column(primary_key)]
    pub id: Text,
    #[column(not_null)]
    pub name: Text,
    #[column(not_null)]
    pub email: Text,
    pub age: Integer,
    pub bio: Text,
  }

  #[table(name = "posts")]
  pub struct Posts {
    #[column(primary_key)]
    pub id: Text,
    #[column(not_null, references = "users(id)", on_delete = "cascade")]
    pub author_id: Text,
    #[column(not_null)]
    pub title: Text,
    #[column(not_null, default = "'active'")]
    pub status: Status,
  }
}

pub fn registry_v1() -> SchemaRegistry {
  SchemaRegistry::from_tables(vec![v1::Users::table_def()])
}

pub fn registry_v2() -> SchemaRegistry {
  SchemaRegistry::from_tables(vec![v2::Users::table_def(), v2::Posts::table_def()])
}

/// A fresh `migrations/` directory inside a tempdir. Keep the `TempDir` alive
/// for the test's duration.
///
/// # Errors
///
/// Returns the tempdir or path error.
pub fn migrations_dir() -> Result<(tempfile::TempDir, String), Box<dyn std::error::Error>> {
  let dir = tempfile::tempdir()?;
  let path = dir.path().join("migrations");
  std::fs::create_dir_all(&path)?;
  let path = path.to_str().ok_or("non-UTF8 tempdir path")?.to_owned();
  Ok((dir, path))
}
