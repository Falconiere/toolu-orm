//! Macro-generated public items carry docs under `#![deny(missing_docs)]`.
//!
//! A consumer that exports `#[table]` / `#[fts5_table]` / `#[vec0_table]`
//! types from a `pub mod` must compile when `missing_docs` is deny. The
//! companion column module, its constants, and the builder methods all come
//! from the macros — this binary is the compile-proof that those expansions
//! emit `#[doc = "..."]`.

#![deny(missing_docs)]

use toolu_orm::core::column::{Text, Vector};
use toolu_orm::{fts5_table, table, vec0_table};

/// Schema types exported under `missing_docs = "deny"`.
pub mod schema {
  use super::*;

  /// Ordinary table used as a missing_docs compile proof.
  #[table(name = "missing_docs_users")]
  pub struct MissingDocsUser {
    /// Primary key.
    #[column(primary_key)]
    pub id: Text,
    /// Display name.
    pub name: Text,
  }

  /// FTS5 virtual table used as a missing_docs compile proof.
  #[fts5_table(name = "missing_docs_user_fts")]
  pub struct MissingDocsUserFts {
    /// Unindexed row id.
    #[column(unindexed)]
    pub user_id: Text,
    /// Searchable body.
    pub body: Text,
  }

  /// vec0 virtual table used as a missing_docs compile proof.
  #[vec0_table(name = "missing_docs_user_vec")]
  pub struct MissingDocsUserVec {
    /// Primary key.
    #[column(primary_key)]
    pub user_id: Text,
    /// Embedding vector.
    #[column(dim = 4, distance_metric = "cosine")]
    pub embedding: Vector,
  }
}

#[test]
fn missing_docs_pub_mod_compiles_with_macro_tables() {
  assert_eq!(schema::missing_docs_users::TABLE, "missing_docs_users");
  assert_eq!(
    schema::missing_docs_user_fts::ALL_COLUMNS,
    &["user_id", "body"]
  );
  assert_eq!(
    schema::missing_docs_user_vec::ALL_COLUMNS,
    &["user_id", "embedding"]
  );
  let _ = schema::MissingDocsUser::select();
  let _ = schema::MissingDocsUserFts::insert();
  let _ = schema::MissingDocsUserVec::update();
  let _ = schema::MissingDocsUser::delete();
}
