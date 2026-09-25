#![cfg(feature = "lancedb")]

use std::{error::Error, path::PathBuf};
use toolu_orm_connection::{LanceConnection, LanceStartupError};

fn pinned_extension() -> Result<PathBuf, Box<dyn Error>> {
  Ok(
    std::env::var_os("LANCE_EXTENSION_PATH")
      .map(PathBuf::from)
      .ok_or("LANCE_EXTENSION_PATH is required for the real Lance startup suite")?,
  )
}

#[test]
fn pinned_extension_prepares_lance_sql_after_startup() -> Result<(), Box<dyn Error>> {
  let fixture = tempfile::tempdir()?;
  let quoted_dir = fixture.path().join("quote's extensions");
  std::fs::create_dir(&quoted_dir)?;
  let extension = quoted_dir.join("lance.duckdb_extension");
  std::fs::copy(pinned_extension()?, &extension)?;
  let original = std::fs::read(&extension)?;
  let namespace = fixture.path().join("not-yet-attached");

  let lance = LanceConnection::open(&extension)?;
  assert!(!namespace.exists(), "startup must not attach a namespace");
  assert_eq!(std::fs::read(&extension)?, original);

  let namespace = namespace.to_str().ok_or("non-UTF-8 test directory")?;
  let namespace = namespace.replace('\'', "''");
  lance.connection().execute_batch(&format!(
    "ATTACH '{namespace}' AS startup_test (TYPE LANCE);\
     CREATE TABLE startup_test.main.items (id BIGINT, label VARCHAR);\
     INSERT INTO startup_test.main.items VALUES (1, 'loaded');"
  ))?;
  let mut statement = lance
    .connection()
    .prepare("SELECT label FROM startup_test.main.items WHERE id = ?")?;
  let label: String = statement.query_row([1_i64], |row| row.get(0))?;
  assert_eq!(label, "loaded");
  assert!(
    fixture
      .path()
      .join("not-yet-attached")
      .join("items.lance")
      .exists()
  );
  Ok(())
}

#[test]
fn absent_extension_is_named_before_namespace_mutation() -> Result<(), Box<dyn Error>> {
  let fixture = tempfile::tempdir()?;
  let missing = fixture.path().join("missing.duckdb_extension");
  let namespace = fixture.path().join("untouched");
  let result = LanceConnection::open(&missing);
  let error = result
    .err()
    .ok_or("missing extension unexpectedly loaded")?;
  assert!(matches!(
    error,
    LanceStartupError::LanceDependencyUnavailable(_)
  ));
  assert!(error.to_string().contains("missing.duckdb_extension"));
  assert!(!namespace.exists());
  Ok(())
}

#[test]
fn directory_extension_path_is_named_before_namespace_mutation() -> Result<(), Box<dyn Error>> {
  let fixture = tempfile::tempdir()?;
  let namespace = fixture.path().join("untouched");
  let result = LanceConnection::open(fixture.path());
  let error = result.err().ok_or("directory unexpectedly loaded")?;
  assert!(matches!(
    error,
    LanceStartupError::LanceDependencyUnavailable(_)
  ));
  assert!(!namespace.exists());
  Ok(())
}

#[cfg(unix)]
#[test]
fn backslash_extension_path_is_rejected_before_loading() -> Result<(), Box<dyn Error>> {
  let fixture = tempfile::tempdir()?;
  let extension = fixture.path().join("lance\\'unsafe.duckdb_extension");
  std::fs::write(&extension, b"not an extension")?;
  let error = LanceConnection::open(&extension)
    .err()
    .ok_or("backslash path unexpectedly accepted")?;
  assert!(matches!(
    error,
    LanceStartupError::LanceDependencyUnavailable(_)
  ));
  assert!(error.to_string().contains("unsupported backslash"));
  Ok(())
}

#[test]
fn corrupt_extension_is_named_before_namespace_mutation() -> Result<(), Box<dyn Error>> {
  let fixture = tempfile::tempdir()?;
  let corrupt = fixture.path().join("lance.duckdb_extension");
  std::fs::write(&corrupt, b"not a DuckDB extension")?;
  let namespace = fixture.path().join("untouched");
  let result = LanceConnection::open(&corrupt);
  let error = result
    .err()
    .ok_or("corrupt extension unexpectedly loaded")?;
  assert!(matches!(
    error,
    LanceStartupError::LanceDependencyUnavailable(_)
  ));
  assert!(!namespace.exists());
  Ok(())
}

#[cfg(unix)]
#[test]
fn non_utf8_extension_path_is_named_before_namespace_mutation() -> Result<(), Box<dyn Error>> {
  use std::os::unix::ffi::OsStringExt;

  let fixture = tempfile::tempdir()?;
  let non_utf8 = fixture
    .path()
    .join(std::ffi::OsString::from_vec(vec![0xff]));
  let namespace = fixture.path().join("untouched");
  let result = LanceConnection::open(&non_utf8);
  let error = result
    .err()
    .ok_or("non-UTF-8 extension unexpectedly loaded")?;
  assert!(matches!(
    error,
    LanceStartupError::LanceDependencyUnavailable(_)
  ));
  assert!(error.to_string().contains("not UTF-8"));
  assert!(!namespace.exists());
  Ok(())
}
