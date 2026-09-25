#[path = "fixtures/lance.rs"]
mod lance;

use duckdb::Connection;
use lance::{load_lance, quoted_path};
use std::error::Error;

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
