//! `#[table]` registries for the row-level-security tests. v1 declares one
//! permissive tenant policy on `docs` (which enables row security); v2 adds an
//! `archived` column, a restrictive `SELECT` policy that hides archived rows,
//! and forces row security onto the owner; v3 drops the declaration entirely.
//!
//! The tenant expression reads `NULLIF(current_setting('app.tenant_id', true), '')`:
//! `current_setting(…, true)` is NULL before a session ever set the value but
//! the empty string after a `SET LOCAL` was reset, and `''::int` is an error.
//! Either way an unset context matches no row.
//!
//! Wired into the RLS test binaries with `#[path]`; every item here is used by
//! each binary that includes it.

use toolu_orm_core::schema::SchemaRegistry;
use toolu_orm_core::table::TableSchema;

pub mod v1 {
  use toolu_orm_core::column::{BigInt, Text};
  use toolu_orm_macros::table;

  #[table(name = "docs")]
  #[policy(
    "tenant_isolation",
    using = "tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::int"
  )]
  pub struct Docs {
    #[column(primary_key)]
    pub id: Text,
    #[column(not_null)]
    pub tenant_id: BigInt,
    #[column(not_null)]
    pub title: Text,
  }
}

pub mod v2 {
  use toolu_orm_core::column::{BigInt, Boolean, Text};
  use toolu_orm_macros::table;

  #[table(name = "docs", rls = "force")]
  #[policy(
    "tenant_isolation",
    using = "tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::int"
  )]
  #[policy("hide_archived", for = select, as = restrictive, using = "NOT archived")]
  pub struct Docs {
    #[column(primary_key)]
    pub id: Text,
    #[column(not_null)]
    pub tenant_id: BigInt,
    #[column(not_null)]
    pub title: Text,
    #[column(not_null, default = "false")]
    pub archived: Boolean,
  }
}

pub mod v3 {
  use toolu_orm_core::column::{BigInt, Boolean, Text};
  use toolu_orm_macros::table;

  #[table(name = "docs")]
  pub struct Docs {
    #[column(primary_key)]
    pub id: Text,
    #[column(not_null)]
    pub tenant_id: BigInt,
    #[column(not_null)]
    pub title: Text,
    #[column(not_null, default = "false")]
    pub archived: Boolean,
  }
}

pub fn registry_v1() -> SchemaRegistry {
  SchemaRegistry::from_tables(vec![v1::Docs::table_def()])
}

pub fn registry_v2() -> SchemaRegistry {
  SchemaRegistry::from_tables(vec![v2::Docs::table_def()])
}

pub fn registry_v3() -> SchemaRegistry {
  SchemaRegistry::from_tables(vec![v3::Docs::table_def()])
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
