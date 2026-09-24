use duckdb::Connection;
use std::{error::Error, fmt, path::Path};

#[derive(Debug)]
struct LanceDependencyUnavailable(String);

impl fmt::Display for LanceDependencyUnavailable {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(formatter, "LanceDependencyUnavailable: {}", self.0)
  }
}

impl Error for LanceDependencyUnavailable {}

fn load_lance(connection: &Connection, path: &Path) -> Result<(), LanceDependencyUnavailable> {
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
    .execute_batch(&format!("LOAD {literal};"))
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

fn quoted_path(path: &Path) -> Result<String, Box<dyn Error>> {
  let path = path.to_str().ok_or("non-UTF-8 Lance directory")?;
  Ok(format!("'{}'", path.replace('\'', "''")))
}

#[test]
fn bound_select_returns_inserted_row_after_reopen() -> Result<(), Box<dyn Error>> {
  let directory = tempfile::tempdir()?;
  let extension = std::env::var_os("LANCE_EXTENSION_PATH")
    .ok_or("LanceDependencyUnavailable: LANCE_EXTENSION_PATH is missing")?;
  let extension = std::path::PathBuf::from(extension);

  {
    let connection = Connection::open_in_memory()?;
    load_lance(&connection, &extension)?;
    connection.execute_batch(&format!(
      "ATTACH {} AS lance_smoke (TYPE LANCE);\
             CREATE TABLE lance_smoke.main.items (id BIGINT, label VARCHAR);\
             INSERT INTO lance_smoke.main.items VALUES (1, 'persisted');",
      quoted_path(directory.path())?
    ))?;
  }

  let reopened = Connection::open_in_memory()?;
  load_lance(&reopened, &extension)?;
  reopened.execute_batch(&format!(
    "ATTACH {} AS lance_smoke (TYPE LANCE);",
    quoted_path(directory.path())?
  ))?;
  let mut statement = reopened.prepare("SELECT label FROM lance_smoke.main.items WHERE id = ?")?;
  let label: String = statement.query_row([1_i64], |row| row.get(0))?;
  assert_eq!(label, "persisted");
  assert!(directory.path().join("items.lance").exists());
  Ok(())
}

#[test]
fn missing_extension_fails_before_table_mutation() -> Result<(), Box<dyn Error>> {
  let directory = tempfile::tempdir()?;
  let nonexistent = directory.path().join("missing.duckdb_extension");
  let connection = Connection::open_in_memory()?;

  let error = load_lance(&connection, &nonexistent).expect_err("missing extension must fail");
  assert_eq!(
    error.to_string(),
    format!(
      "LanceDependencyUnavailable: lance extension file is unavailable: {}",
      nonexistent.display()
    )
  );
  assert_eq!(directory.path().read_dir()?.count(), 0);
  Ok(())
}
