//! `#[table]` registries for the SQLite table-rebuild tests.
//!
//! Every version keeps the same `users`/`posts` pair so a diff between two of
//! them forces exactly the rebuild under test:
//!
//! - **v1 → v2:** `users.name` nullable → `NOT NULL`. One rebuild, no other op.
//! - **v1 → v3:** the same plus a new `age` column (alter + add).
//! - **v1 → v4:** the same plus dropping `bio` (alter + drop).
//!
//! `posts.author_id` references `users(id)`; the cascading variant is the
//! issue's reproduction, the plain one covers `NO ACTION`. `users` also carries
//! a unique index the diff never touches, so a rebuild that forgets to
//! re-create it is visible.
//!
//! Wired into the `sqlite_rebuild_*` binaries with `#[path]`.

use toolu_orm_core::schema::SchemaRegistry;
use toolu_orm_core::table::{TableDef, TableSchema};

pub mod users_v1 {
  use toolu_orm_core::column::Text;
  use toolu_orm_macros::table;

  #[table(name = "users")]
  #[unique_index("idx_users_email", email)]
  pub struct Users {
    #[column(primary_key)]
    pub id: Text,
    pub name: Text,
    #[column(not_null)]
    pub email: Text,
    pub bio: Text,
  }
}

pub mod users_v2 {
  use toolu_orm_core::column::Text;
  use toolu_orm_macros::table;

  #[table(name = "users")]
  #[unique_index("idx_users_email", email)]
  pub struct Users {
    #[column(primary_key)]
    pub id: Text,
    #[column(not_null)]
    pub name: Text,
    #[column(not_null)]
    pub email: Text,
    pub bio: Text,
  }
}

pub mod users_v3 {
  use toolu_orm_core::column::{Integer, Text};
  use toolu_orm_macros::table;

  #[table(name = "users")]
  #[unique_index("idx_users_email", email)]
  pub struct Users {
    #[column(primary_key)]
    pub id: Text,
    #[column(not_null)]
    pub name: Text,
    #[column(not_null)]
    pub email: Text,
    pub bio: Text,
    #[column(default = "7")]
    pub age: Integer,
  }
}

pub mod users_v4 {
  use toolu_orm_core::column::Text;
  use toolu_orm_macros::table;

  #[table(name = "users")]
  #[unique_index("idx_users_email", email)]
  pub struct Users {
    #[column(primary_key)]
    pub id: Text,
    #[column(not_null)]
    pub name: Text,
    #[column(not_null)]
    pub email: Text,
  }
}

pub mod posts_cascade {
  use toolu_orm_core::column::Text;
  use toolu_orm_macros::table;

  #[table(name = "posts")]
  pub struct Posts {
    #[column(primary_key)]
    pub id: Text,
    #[column(not_null, references = "users(id)", on_delete = "cascade")]
    pub author_id: Text,
    #[column(not_null)]
    pub title: Text,
  }
}

pub mod posts_no_action {
  use toolu_orm_core::column::Text;
  use toolu_orm_macros::table;

  #[table(name = "posts")]
  pub struct Posts {
    #[column(primary_key)]
    pub id: Text,
    #[column(not_null, references = "users(id)")]
    pub author_id: Text,
    #[column(not_null)]
    pub title: Text,
  }
}

fn cascading(users: TableDef) -> SchemaRegistry {
  SchemaRegistry::from_tables(vec![users, posts_cascade::Posts::table_def()])
}

/// `users.name` nullable, with a cascading child.
pub fn registry_v1() -> SchemaRegistry {
  cascading(users_v1::Users::table_def())
}

/// `users.name` NOT NULL — the one change that forces the rebuild.
pub fn registry_v2() -> SchemaRegistry {
  cascading(users_v2::Users::table_def())
}

/// v2 plus a new `age INTEGER DEFAULT (7)`: alter and add in one diff.
pub fn registry_v3() -> SchemaRegistry {
  cascading(users_v3::Users::table_def())
}

/// v2 minus `bio`: alter and drop in one diff.
pub fn registry_v4() -> SchemaRegistry {
  cascading(users_v4::Users::table_def())
}

/// v1 with a child whose foreign key has no `ON DELETE` action.
pub fn registry_no_action_v1() -> SchemaRegistry {
  SchemaRegistry::from_tables(vec![
    users_v1::Users::table_def(),
    posts_no_action::Posts::table_def(),
  ])
}

/// v2 with a child whose foreign key has no `ON DELETE` action.
pub fn registry_no_action_v2() -> SchemaRegistry {
  SchemaRegistry::from_tables(vec![
    users_v2::Users::table_def(),
    posts_no_action::Posts::table_def(),
  ])
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
