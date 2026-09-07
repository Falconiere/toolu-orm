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
//! ## Macro paths
//!
//! The proc macros expand to absolute paths resolved against the consuming
//! crate's `Cargo.toml`: `::toolu_orm::core::…` when this facade is the
//! dependency, `::toolu_orm_core::…` when the crates are named directly. So
//! `toolu-orm` on its own is enough — import the macro and nothing else:
//!
//! ```ignore
//! use toolu_orm::table;
//!
//! #[table(name = "users")]
//! pub struct User {
//!   pub id: i64,
//!   pub email: String,
//! }
//! ```
//!
//! The [`prelude`] remains for code that names `toolu_orm_core`,
//! `toolu_orm_query` or the driver crate itself; the macros do not need it.

pub use toolu_orm_connection as connection;
pub use toolu_orm_core as core;
pub use toolu_orm_query as query;

pub use toolu_orm_macros::{table, ColumnEnum, FromRow, Relational};

/// The macros plus the crate names they used to require.
///
/// The expansions resolve on their own now, so this is a convenience: glob it
/// when your own code wants to write `toolu_orm_core::…`, `toolu_orm_query::…`
/// or the driver crate for the enabled feature without naming the facade path
/// each time.
pub mod prelude {
  pub use toolu_orm_core;
  pub use toolu_orm_query;

  pub use toolu_orm_macros::{table, ColumnEnum, FromRow, Relational};

  #[cfg(feature = "libsql")]
  pub use toolu_orm_core::libsql;
  #[cfg(feature = "rusqlite")]
  pub use toolu_orm_core::rusqlite;
  #[cfg(feature = "postgres")]
  pub use toolu_orm_core::tokio_postgres;
}
