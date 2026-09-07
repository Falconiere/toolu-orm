//! Single entry point for the toolu-orm workspace.
//!
//! `toolu-orm` is a facade: it contains no logic of its own, only re-exports
//! of the four library crates, all pinned to one version.
//!
//! ```toml
//! toolu-orm = { version = "0.1", features = ["postgres"] }
//! ```
//!
//! | Re-export | Crate | Holds |
//! |---|---|---|
//! | [`core`] | `toolu-orm-core` | schema, columns, migrations, driver traits |
//! | [`query`] | `toolu-orm-query` | select / insert / update / delete builders |
//! | [`connection`] | `toolu-orm-connection` | pools and driver adapters |
//! | [`table`], [`ColumnEnum`], [`FromRow`], [`Relational`] | `toolu-orm-macros` | proc macros |
//!
//! ## Features
//!
//! `libsql`, `rusqlite` and `postgres` forward to every re-exported crate, so
//! one feature list drives the whole stack. Enable exactly one: `toolu-orm-query`
//! compiles its executor and transaction code only for a single driver.
//!
//! ## Prelude
//!
//! The proc macros expand to paths that name `toolu_orm_core` (and, for the
//! generated query builders, `toolu_orm_query`) directly. Depending on
//! `toolu-orm` alone does not put those crate names in scope, so import the
//! [`prelude`] in every module that uses `#[table]` or a derive:
//!
//! ```ignore
//! use toolu_orm::prelude::*;
//!
//! #[table("users")]
//! pub struct User {
//!   pub id: i64,
//!   pub email: String,
//! }
//! ```

pub use toolu_orm_connection as connection;
pub use toolu_orm_core as core;
pub use toolu_orm_query as query;

pub use toolu_orm_macros::{table, ColumnEnum, FromRow, Relational};

/// Everything a module needs to expand `#[table]` and the derives.
///
/// Glob-import this. Beyond the macros themselves it re-exports the crate
/// names the expansions refer to — `toolu_orm_core`, `toolu_orm_query`, and
/// the driver crate for the enabled feature — which a dependency on
/// `toolu-orm` alone would not bring into scope.
pub mod prelude {
  pub use toolu_orm_core;
  pub use toolu_orm_query;

  pub use toolu_orm_macros::{table, ColumnEnum, FromRow, Relational};

  #[cfg(feature = "libsql")]
  pub use toolu_orm_core::libsql;
  #[cfg(feature = "postgres")]
  pub use toolu_orm_core::tokio_postgres;
  #[cfg(feature = "rusqlite")]
  pub use toolu_orm_core::rusqlite;
}
