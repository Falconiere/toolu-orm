//! A selected local Lance catalog and its table lifecycle.

use duckdb::Connection;
use std::path::Path;

use super::sql::{quoted_identifier, quoted_path};

/// A supported column type for a lifecycle table.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LanceColumnType {
  /// Signed 64-bit integer.
  BigInt,
  /// UTF-8 text.
  Varchar,
}

impl LanceColumnType {
  fn sql(self) -> &'static str {
    match self {
      Self::BigInt => "BIGINT",
      Self::Varchar => "VARCHAR",
    }
  }
}

/// A column in a newly created local Lance table.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LanceColumn<'a> {
  /// Column name, quoted as a SQL identifier.
  pub name: &'a str,
  /// Type supported by this lifecycle API.
  pub data_type: LanceColumnType,
}

/// Errors while attaching a local namespace or changing its tables.
#[derive(Debug, thiserror::Error)]
pub enum LanceNamespaceError {
  /// The directory is missing or cannot be represented as a safe SQL literal.
  #[error("LanceInvalidPath: {0}")]
  InvalidPath(String),
  /// A namespace, table, or column name is invalid.
  #[error("LanceInvalidIdentifier: {0}")]
  InvalidIdentifier(String),
  /// Creating a table requires at least one column.
  #[error("LanceEmptySchema: table must contain at least one column")]
  EmptySchema,
  /// Two columns use the same name under DuckDB's case-insensitive lookup.
  #[error("LanceDuplicateColumn: {0}")]
  DuplicateColumn(String),
  /// Creation refused an existing table without replacing it.
  #[error("LanceTableAlreadyExists: {0}")]
  TableAlreadyExists(String),
  /// Opening or dropping an unknown table failed without changing other tables.
  #[error("LanceTableNotFound: {0}")]
  TableNotFound(String),
  /// DuckDB rejected the namespace attachment.
  #[error("LanceAttach: {0}")]
  Attach(#[source] duckdb::Error),
  /// DuckDB could not select the attached catalog.
  #[error("LanceUse: {0}")]
  Use(#[source] duckdb::Error),
  /// DuckDB could not list tables in the attached catalog.
  #[error("LanceListTables: {0}")]
  ListTables(#[source] duckdb::Error),
  /// DuckDB rejected table creation.
  #[error("LanceCreateTable: {0}")]
  CreateTable(#[source] duckdb::Error),
  /// DuckDB rejected table deletion.
  #[error("LanceDropTable: {0}")]
  DropTable(#[source] duckdb::Error),
}

/// A DuckDB connection with one local Lance catalog selected for unqualified SQL.
pub struct LanceNamespace {
  connection: Connection,
  catalog: String,
}

impl LanceNamespace {
  /// Attach and select a local catalog after the extension has loaded.
  pub(super) fn attach(
    connection: Connection,
    directory: &Path,
    namespace: &str,
  ) -> Result<Self, LanceNamespaceError> {
    let path = quoted_path(directory)?;
    let catalog = quoted_identifier(namespace)?;
    connection
      .execute_batch(&format!("ATTACH {path} AS {catalog} (TYPE LANCE)"))
      .map_err(LanceNamespaceError::Attach)?;
    connection
      .execute_batch(&format!("USE {catalog}"))
      .map_err(LanceNamespaceError::Use)?;
    Ok(Self {
      connection,
      catalog: namespace.into(),
    })
  }

  /// Borrow the selected connection for direct SQL with unqualified table names.
  #[must_use]
  pub fn connection(&self) -> &Connection {
    &self.connection
  }

  /// Return names of tables in this Lance catalog's `main` schema.
  ///
  /// # Errors
  ///
  /// Returns [`LanceNamespaceError::ListTables`] if DuckDB cannot inspect it.
  pub fn list_tables(&self) -> Result<Vec<String>, LanceNamespaceError> {
    let catalog = quoted_identifier(&self.catalog)?;
    let sql = format!("SHOW TABLES FROM {catalog}.main");
    let mut statement = self
      .connection
      .prepare(&sql)
      .map_err(LanceNamespaceError::ListTables)?;
    let rows = statement
      .query_map([], |row| row.get(0))
      .map_err(LanceNamespaceError::ListTables)?;
    let mut names: Vec<String> = rows
      .collect::<duckdb::Result<Vec<String>>>()
      .map_err(LanceNamespaceError::ListTables)?;
    names.sort();
    Ok(names)
  }

  /// Check that a real table exists without creating or changing it.
  ///
  /// # Errors
  ///
  /// Returns [`LanceNamespaceError::TableNotFound`] for an absent table, or a
  /// validation or catalog error when lookup cannot complete.
  pub fn open_table(&self, name: &str) -> Result<(), LanceNamespaceError> {
    let _identifier = quoted_identifier(name)?;
    if self.has_table(name)? {
      Ok(())
    } else {
      Err(LanceNamespaceError::TableNotFound(name.into()))
    }
  }

  /// Create an empty real Lance table without replacing an existing dataset.
  ///
  /// # Errors
  ///
  /// Returns a named duplicate, invalid-schema, validation, or DuckDB error.
  pub fn create_table(
    &self,
    name: &str,
    columns: &[LanceColumn<'_>],
  ) -> Result<(), LanceNamespaceError> {
    let table = quoted_identifier(name)?;
    if columns.is_empty() {
      return Err(LanceNamespaceError::EmptySchema);
    }
    let mut definitions = Vec::with_capacity(columns.len());
    let mut seen: Vec<&str> = Vec::with_capacity(columns.len());
    for column in columns {
      let identifier = quoted_identifier(column.name)?;
      if seen
        .iter()
        .any(|other| other.eq_ignore_ascii_case(column.name))
      {
        return Err(LanceNamespaceError::DuplicateColumn(column.name.into()));
      }
      seen.push(column.name);
      definitions.push(format!("{identifier} {}", column.data_type.sql()));
    }
    if self.has_table(name)? {
      return Err(LanceNamespaceError::TableAlreadyExists(name.into()));
    }
    let catalog = quoted_identifier(&self.catalog)?;
    let sql = format!(
      "CREATE TABLE {catalog}.main.{table} ({})",
      definitions.join(", ")
    );
    self
      .connection
      .execute_batch(&sql)
      .map_err(LanceNamespaceError::CreateTable)
  }

  /// Drop exactly one existing Lance table.
  ///
  /// # Errors
  ///
  /// Returns [`LanceNamespaceError::TableNotFound`] for an absent table, or a
  /// validation or DuckDB error. No `IF EXISTS` behavior is applied.
  pub fn drop_table(&self, name: &str) -> Result<(), LanceNamespaceError> {
    let table = quoted_identifier(name)?;
    if !self.has_table(name)? {
      return Err(LanceNamespaceError::TableNotFound(name.into()));
    }
    let catalog = quoted_identifier(&self.catalog)?;
    self
      .connection
      .execute_batch(&format!("DROP TABLE {catalog}.main.{table}"))
      .map_err(LanceNamespaceError::DropTable)
  }

  fn has_table(&self, name: &str) -> Result<bool, LanceNamespaceError> {
    Ok(
      self
        .list_tables()?
        .iter()
        .any(|table| table.eq_ignore_ascii_case(name)),
    )
  }
}
