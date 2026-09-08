//! `FromRow` for `#[derive(FromRow)]`, in whatever shape this build compiled.
//!
//! The derive cannot see which driver features Cargo unified onto
//! `toolu-orm-core`. It expands inside the consumer's crate, where
//! `feature = "postgres"` names the *consumer's* feature, and the
//! `orm_core_has_*` cfgs that `orm-cli`'s build script derives from
//! `DEP_TOOLU_ORM_CORE_HAS_*` do not exist — a build script and a direct
//! `toolu-orm-core` dependency are exactly what an arbitrary expansion site
//! lacks.
//!
//! A `macro_rules!` definition, though, is compiled with its *defining* crate's
//! features. So the eight definitions below are gated on the same eight
//! partitions as the trait in [`super::traits`], and only the one matching the
//! shape this build of `toolu-orm-core` compiled survives. What it expands to
//! carries no `cfg` at all, which is why the consumer needs neither a build
//! script nor a registered cfg to derive `FromRow`.
//!
//! Each decoder arrives as `|row| { … }` rather than as a body alone, because
//! the caller's block names the row: `macro_rules!` hygiene keeps a `row` this
//! macro declared itself invisible to tokens the derive produced. Taking the
//! binding as an `ident` puts both in the caller's hygiene context. That is
//! also why these arms emit the impl directly instead of delegating to
//! [`impl_from_row_for!`](crate::impl_from_row_for), which names its own `row`
//! and so only accepts a callable.

/// Implements [`FromRow`](super::FromRow) for `$ty` from one decoder per
/// driver, in this build's shape — `postgres` only.
///
/// Each decoder is `|row| { … }` evaluating to
/// `Result<$ty, DbCoreError>`. Decoders for inactive drivers are matched and
/// dropped without ever being expanded, so naming `tokio_postgres::Row` in a
/// rusqlite-only build costs nothing.
///
/// This is `#[derive(FromRow)]`'s expansion target, not a hand-writing API:
/// implement the trait directly, or use
/// [`impl_from_row_for!`](crate::impl_from_row_for), when spelling one out.
#[cfg(all(
  feature = "postgres",
  not(feature = "libsql"),
  not(feature = "rusqlite"),
))]
#[macro_export]
macro_rules! impl_derived_from_row {
  ($ty:ty, $cols:expr, postgres = |$p:ident| $pb:block,
    libsql = |$l:ident| $lb:block, rusqlite = |$r:ident| $rb:block $(,)?) => {
    impl $crate::row::FromRow for $ty {
      const REQUIRED_COLUMNS: &'static [&'static str] = $cols;
      fn from_row($p: &$crate::tokio_postgres::Row)
        -> Result<Self, $crate::error::DbCoreError> $pb
    }
  };
}

/// [`impl_derived_from_row!`] for a `libsql`-only build.
#[cfg(all(
  feature = "libsql",
  not(feature = "postgres"),
  not(feature = "rusqlite"),
))]
#[macro_export]
macro_rules! impl_derived_from_row {
  ($ty:ty, $cols:expr, postgres = |$p:ident| $pb:block,
    libsql = |$l:ident| $lb:block, rusqlite = |$r:ident| $rb:block $(,)?) => {
    impl $crate::row::FromRow for $ty {
      const REQUIRED_COLUMNS: &'static [&'static str] = $cols;
      fn from_row($l: &$crate::libsql::Row) -> Result<Self, $crate::error::DbCoreError> $lb
    }
  };
}

/// [`impl_derived_from_row!`] for a `rusqlite`-only build.
#[cfg(all(
  feature = "rusqlite",
  not(feature = "postgres"),
  not(feature = "libsql"),
))]
#[macro_export]
macro_rules! impl_derived_from_row {
  ($ty:ty, $cols:expr, postgres = |$p:ident| $pb:block,
    libsql = |$l:ident| $lb:block, rusqlite = |$r:ident| $rb:block $(,)?) => {
    impl $crate::row::FromRow for $ty {
      const REQUIRED_COLUMNS: &'static [&'static str] = $cols;
      fn from_row($r: &$crate::rusqlite::Row<'_>) -> Result<Self, $crate::error::DbCoreError> $rb
    }
  };
}

/// [`impl_derived_from_row!`] for a `postgres` + `libsql` build.
#[cfg(all(feature = "postgres", feature = "libsql", not(feature = "rusqlite"),))]
#[macro_export]
macro_rules! impl_derived_from_row {
  ($ty:ty, $cols:expr, postgres = |$p:ident| $pb:block,
    libsql = |$l:ident| $lb:block, rusqlite = |$r:ident| $rb:block $(,)?) => {
    impl $crate::row::FromRow for $ty {
      const REQUIRED_COLUMNS: &'static [&'static str] = $cols;
      fn from_pg_row($p: &$crate::tokio_postgres::Row)
        -> Result<Self, $crate::error::DbCoreError> $pb
      fn from_libsql_row($l: &$crate::libsql::Row)
        -> Result<Self, $crate::error::DbCoreError> $lb
    }
  };
}

/// [`impl_derived_from_row!`] for a `postgres` + `rusqlite` build.
#[cfg(all(feature = "postgres", feature = "rusqlite", not(feature = "libsql"),))]
#[macro_export]
macro_rules! impl_derived_from_row {
  ($ty:ty, $cols:expr, postgres = |$p:ident| $pb:block,
    libsql = |$l:ident| $lb:block, rusqlite = |$r:ident| $rb:block $(,)?) => {
    impl $crate::row::FromRow for $ty {
      const REQUIRED_COLUMNS: &'static [&'static str] = $cols;
      fn from_pg_row($p: &$crate::tokio_postgres::Row)
        -> Result<Self, $crate::error::DbCoreError> $pb
      fn from_rusqlite_row($r: &$crate::rusqlite::Row<'_>)
        -> Result<Self, $crate::error::DbCoreError> $rb
    }
  };
}

/// [`impl_derived_from_row!`] for a `libsql` + `rusqlite` build.
#[cfg(all(feature = "libsql", feature = "rusqlite", not(feature = "postgres"),))]
#[macro_export]
macro_rules! impl_derived_from_row {
  ($ty:ty, $cols:expr, postgres = |$p:ident| $pb:block,
    libsql = |$l:ident| $lb:block, rusqlite = |$r:ident| $rb:block $(,)?) => {
    impl $crate::row::FromRow for $ty {
      const REQUIRED_COLUMNS: &'static [&'static str] = $cols;
      fn from_libsql_row($l: &$crate::libsql::Row)
        -> Result<Self, $crate::error::DbCoreError> $lb
      fn from_rusqlite_row($r: &$crate::rusqlite::Row<'_>)
        -> Result<Self, $crate::error::DbCoreError> $rb
    }
  };
}

/// [`impl_derived_from_row!`] for a build with all three drivers.
#[cfg(all(feature = "postgres", feature = "libsql", feature = "rusqlite",))]
#[macro_export]
macro_rules! impl_derived_from_row {
  ($ty:ty, $cols:expr, postgres = |$p:ident| $pb:block,
    libsql = |$l:ident| $lb:block, rusqlite = |$r:ident| $rb:block $(,)?) => {
    impl $crate::row::FromRow for $ty {
      const REQUIRED_COLUMNS: &'static [&'static str] = $cols;
      fn from_pg_row($p: &$crate::tokio_postgres::Row)
        -> Result<Self, $crate::error::DbCoreError> $pb
      fn from_libsql_row($l: &$crate::libsql::Row)
        -> Result<Self, $crate::error::DbCoreError> $lb
      fn from_rusqlite_row($r: &$crate::rusqlite::Row<'_>)
        -> Result<Self, $crate::error::DbCoreError> $rb
    }
  };
}

/// [`impl_derived_from_row!`] for a build with no driver: the fallback trait
/// has no decoding method, so only `REQUIRED_COLUMNS` is emitted and all three
/// decoders are dropped.
#[cfg(not(any(
  all(
    feature = "postgres",
    not(feature = "libsql"),
    not(feature = "rusqlite")
  ),
  all(
    feature = "libsql",
    not(feature = "postgres"),
    not(feature = "rusqlite")
  ),
  all(
    feature = "rusqlite",
    not(feature = "postgres"),
    not(feature = "libsql")
  ),
  all(feature = "postgres", feature = "libsql", not(feature = "rusqlite")),
  all(feature = "postgres", feature = "rusqlite", not(feature = "libsql")),
  all(feature = "libsql", feature = "rusqlite", not(feature = "postgres")),
  all(feature = "postgres", feature = "libsql", feature = "rusqlite"),
)))]
#[macro_export]
macro_rules! impl_derived_from_row {
  ($ty:ty, $cols:expr, postgres = |$p:ident| $pb:block,
    libsql = |$l:ident| $lb:block, rusqlite = |$r:ident| $rb:block $(,)?) => {
    impl $crate::row::FromRow for $ty {
      const REQUIRED_COLUMNS: &'static [&'static str] = $cols;
    }
  };
}
