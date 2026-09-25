use duckdb::Connection;
use std::{error::Error, path::Path};

pub fn quoted(path: &Path) -> Result<String, Box<dyn Error>> {
  let path = path.to_str().ok_or("non-UTF-8 Lance path")?;
  Ok(format!("'{}'", path.replace('\'', "''")))
}

pub fn open(directory: &Path) -> Result<Connection, Box<dyn Error>> {
  let extension = std::env::var_os("LANCE_EXTENSION_PATH")
    .ok_or("LanceDependencyUnavailable: LANCE_EXTENSION_PATH is missing")?;
  let extension = std::path::PathBuf::from(extension);
  if !extension.is_file() {
    return Err(format!("LanceDependencyUnavailable: {}", extension.display()).into());
  }
  let connection = Connection::open_in_memory()?;
  let version: String = connection.query_row("SELECT version()", [], |row| row.get(0))?;
  assert_eq!(version, "v1.5.5");
  connection.execute_batch(&format!("LOAD {}", quoted(&extension)?))?;
  let extension_version: String = connection.query_row(
    "SELECT extension_version FROM duckdb_extensions() WHERE extension_name = 'lance'",
    [],
    |row| row.get(0),
  )?;
  assert_eq!(extension_version, "2f167ea");
  connection.execute_batch(&format!(
    "ATTACH {} AS lance_probe (TYPE LANCE)",
    quoted(directory)?
  ))?;
  Ok(connection)
}

pub fn trial(connection: &Connection, label: &str, sql: &str) {
  match connection.execute_batch(sql) {
    Ok(()) => println!("{label} OK | {sql}"),
    Err(error) => panic!("{label} failed: {error} | {sql}"),
  }
}

pub fn names(connection: &Connection, sql: &str) -> duckdb::Result<Vec<String>> {
  let mut statement = connection.prepare(sql)?;
  statement.query_map([], |row| row.get(0))?.collect()
}

pub fn refusal(connection: &Connection, sql: &str, fragment: &str) {
  match connection.execute_batch(sql) {
    Ok(()) => panic!("unexpectedly accepted: {sql}"),
    Err(error) => assert!(
      error.to_string().contains(fragment),
      "SQL: {sql}; expected error containing {fragment:?}; got {error}"
    ),
  }
}

pub fn pairs(connection: &Connection, sql: &str) -> duckdb::Result<Vec<(i64, f64)>> {
  let mut statement = connection.prepare(sql)?;
  statement
    .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
    .collect()
}
