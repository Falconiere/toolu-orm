use duckdb::Connection;
use std::{error::Error, fmt, path::Path};

#[derive(Debug)]
pub struct LanceDependencyUnavailable(String);

impl fmt::Display for LanceDependencyUnavailable {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(formatter, "LanceDependencyUnavailable: {}", self.0)
  }
}

impl Error for LanceDependencyUnavailable {}

pub fn load_lance(connection: &Connection, path: &Path) -> Result<(), LanceDependencyUnavailable> {
  let engine: String = connection
    .query_row("SELECT version()", [], |row| row.get(0))
    .map_err(|error| LanceDependencyUnavailable(format!("DuckDB version: {error}")))?;
  if engine != "v1.5.5" {
    return Err(LanceDependencyUnavailable(format!(
      "lance requires DuckDB v1.5.5, found {engine}"
    )));
  }
  if !path.is_file() {
    return Err(LanceDependencyUnavailable(format!(
      "lance extension file is unavailable: {}",
      path.display()
    )));
  }

  let literal = quoted_path(path)
    .map_err(|error| LanceDependencyUnavailable(format!("lance extension path: {error}")))?;
  connection
    .execute(&format!("LOAD {literal}"), [])
    .map_err(|error| LanceDependencyUnavailable(format!("lance extension load: {error}")))?;

  let (version, loaded): (String, bool) = connection
    .query_row(
      "SELECT extension_version, loaded FROM duckdb_extensions() WHERE extension_name = 'lance'",
      [],
      |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .map_err(|error| LanceDependencyUnavailable(format!("lance extension metadata: {error}")))?;
  if version != "2f167ea" || !loaded {
    return Err(LanceDependencyUnavailable(format!(
      "lance extension version mismatch: expected loaded 2f167ea, found {version}, loaded={loaded}"
    )));
  }
  println!("DuckDB {engine}; Lance extension {version}");
  Ok(())
}

pub fn quoted_path(path: &Path) -> Result<String, Box<dyn Error>> {
  let path = path.to_str().ok_or("non-UTF-8 Lance directory")?;
  Ok(format!("'{}'", path.replace('\'', "''")))
}
