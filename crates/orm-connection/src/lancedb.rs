//! Startup for an embedded DuckDB connection with the pinned Lance extension.

use duckdb::Connection;
use std::path::Path;

const DUCKDB_VERSION: &str = "v1.5.5";
const LANCE_VERSION: &str = "2f167ea";

/// Startup failures before a Lance namespace or table can be touched.
#[derive(Debug, thiserror::Error)]
pub enum LanceStartupError {
  /// Embedded DuckDB could not be opened.
  #[error("DuckDbOpen: {0}")]
  DuckDbOpen(#[source] duckdb::Error),

  /// The local Lance extension is absent, incompatible, or could not load.
  #[error("LanceDependencyUnavailable: {0}")]
  LanceDependencyUnavailable(String),
}

/// An in-memory DuckDB connection with the pinned Lance extension loaded.
///
/// Startup does not attach a Lance directory or create any tables. Callers can
/// use [`Self::connection`] to prepare SQL after they attach a namespace.
pub struct LanceConnection {
  connection: Connection,
}

impl LanceConnection {
  /// Open embedded DuckDB and load a previously provisioned local extension.
  ///
  /// The caller owns artifact download and caching. This method verifies the
  /// DuckDB engine and the loaded Lance build on every new connection.
  ///
  /// # Errors
  ///
  /// Returns [`LanceStartupError::DuckDbOpen`] if DuckDB cannot open, or
  /// [`LanceStartupError::LanceDependencyUnavailable`] for an absent or
  /// incompatible extension. No namespace or table is changed on failure.
  pub fn open(extension_path: impl AsRef<Path>) -> Result<Self, LanceStartupError> {
    let path = extension_path.as_ref();
    let utf8_path = path.to_str().ok_or_else(|| {
      LanceStartupError::LanceDependencyUnavailable("lance extension path is not UTF-8".into())
    })?;
    if !path.is_file() {
      return Err(LanceStartupError::LanceDependencyUnavailable(format!(
        "lance extension file is unavailable: {}",
        path.display()
      )));
    }

    let connection = Connection::open_in_memory().map_err(LanceStartupError::DuckDbOpen)?;
    let engine: String = connection
      .query_row("SELECT version()", [], |row| row.get(0))
      .map_err(|error| {
        LanceStartupError::LanceDependencyUnavailable(format!("DuckDB version: {error}"))
      })?;
    if engine != DUCKDB_VERSION {
      return Err(LanceStartupError::LanceDependencyUnavailable(format!(
        "lance requires DuckDB {DUCKDB_VERSION}, found {engine}"
      )));
    }

    let literal = format!("'{}'", utf8_path.replace('\'', "''"));
    connection
      .execute(&format!("LOAD {literal}"), [])
      .map_err(|error| {
        LanceStartupError::LanceDependencyUnavailable(format!("lance extension load: {error}"))
      })?;

    let (version, loaded): (String, bool) = connection
      .query_row(
        "SELECT extension_version, loaded FROM duckdb_extensions() WHERE extension_name = 'lance'",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
      )
      .map_err(|error| {
        LanceStartupError::LanceDependencyUnavailable(format!("lance extension metadata: {error}"))
      })?;
    if version != LANCE_VERSION || !loaded {
      return Err(LanceStartupError::LanceDependencyUnavailable(format!(
        "lance extension version mismatch: expected loaded {LANCE_VERSION}, found {version}, loaded={loaded}"
      )));
    }

    Ok(Self { connection })
  }

  /// Borrow the loaded DuckDB connection for SQL preparation and execution.
  #[must_use]
  pub fn connection(&self) -> &Connection {
    &self.connection
  }
}
