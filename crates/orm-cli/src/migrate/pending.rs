//! Pending migration detection by comparing applied records against the migrations directory.

use std::path::Path;

use super::error::MigrateError;

pub(crate) fn get_pending_migrations(
  migrations_dir: &str,
  applied: &[String],
) -> Result<Vec<String>, MigrateError> {
  let path = Path::new(migrations_dir);
  if !path.exists() {
    return Ok(vec![]);
  }

  let mut files: Vec<String> = Vec::new();
  let entries =
    std::fs::read_dir(path).map_err(|e| MigrateError::ReadDir(format!("{migrations_dir}: {e}")))?;

  for entry in entries {
    let entry = entry.map_err(|e| MigrateError::ReadDir(format!("{migrations_dir}: {e}")))?;
    let name = entry.file_name().to_string_lossy().into_owned();
    if name.ends_with(".sql") && !applied.contains(&name) {
      files.push(name);
    }
  }

  files.sort();
  Ok(files)
}
