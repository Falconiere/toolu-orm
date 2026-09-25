use std::path::Path;

use super::LanceNamespaceError;

/// Validate a local directory and render it as one SQL string literal.
pub(super) fn quoted_path(path: &Path) -> Result<String, LanceNamespaceError> {
  if !path.is_dir() {
    return Err(LanceNamespaceError::InvalidPath(format!(
      "Lance namespace directory does not exist or is not a directory: {}",
      path.display()
    )));
  }
  let text = path.to_str().ok_or_else(|| {
    LanceNamespaceError::InvalidPath("Lance namespace path is not valid UTF-8".into())
  })?;
  if text.contains(['\\', '\0']) {
    return Err(LanceNamespaceError::InvalidPath(
      "Lance namespace path contains an unsupported backslash or NUL".into(),
    ));
  }
  Ok(format!("'{}'", text.replace('\'', "''")))
}

/// Validate a name and render it as one SQL identifier.
pub(super) fn quoted_identifier(name: &str) -> Result<String, LanceNamespaceError> {
  let mut characters = name.chars();
  if !characters
    .next()
    .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
    || !characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
  {
    return Err(LanceNamespaceError::InvalidIdentifier(name.into()));
  }
  Ok(format!("\"{name}\""))
}
