//! Dialect enum (Postgres/SQLite) and compile-time dispatch.

/// SQL dialect for code generation.
///
/// Selected at compile time via the `postgres` feature flag.
/// [`Dialect::CURRENT`] resolves to the active dialect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
  Sqlite,
  Postgres,
}

impl Dialect {
  #[must_use]
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::Sqlite => "sqlite",
      Self::Postgres => "postgres",
    }
  }

  /// The dialect selected at compile time.
  #[cfg(feature = "postgres")]
  pub const CURRENT: Dialect = Dialect::Postgres;

  /// The dialect selected at compile time.
  #[cfg(not(feature = "postgres"))]
  pub const CURRENT: Dialect = Dialect::Sqlite;

  /// Positional parameter placeholder for this dialect.
  ///
  /// SQLite uses `?N`, Postgres uses `$N`.
  pub fn param(self, index: usize) -> String {
    match self {
      Self::Sqlite => format!("?{index}"),
      Self::Postgres => format!("${index}"),
    }
  }

  /// Translate SQLite-specific default expressions to their Postgres equivalents.
  ///
  /// Unknown defaults pass through unchanged.
  pub fn map_default(self, default: &str) -> String {
    match self {
      Self::Sqlite => default.to_owned(),
      Self::Postgres => match default {
        "unixepoch()" => "extract(epoch from now())::bigint".to_owned(),
        "uuid4_str()" => "gen_random_uuid()".to_owned(),
        other => other.to_owned(),
      },
    }
  }

  /// SQL expression for the current Unix epoch in seconds.
  ///
  /// Postgres: `extract(epoch from now())::bigint`; SQLite/libsql: `unixepoch()`.
  /// Use this instead of hardcoding the Postgres form in raw SQL / `set_expr`.
  #[must_use]
  pub const fn now_epoch(self) -> &'static str {
    match self {
      Self::Sqlite => "unixepoch()",
      Self::Postgres => "extract(epoch from now())::bigint",
    }
  }

  /// Quote a SQL identifier (table name, column name).
  ///
  /// Both SQLite and Postgres use double quotes for identifiers.
  pub fn quote_ident(self, ident: &str) -> String {
    format!(r#""{ident}""#)
  }
}
