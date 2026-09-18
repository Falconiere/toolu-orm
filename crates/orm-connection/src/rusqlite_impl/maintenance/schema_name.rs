//! Attachment identifiers: what this API accepts, and how it renders them.
//!
//! `ATTACH DATABASE <file> AS <schema>` takes the file as an expression — so it
//! is always a bound parameter here — but the schema is an *identifier*, which
//! SQLite will not accept as a parameter. It therefore has to be written into
//! the statement text, and this module is the only place that does it.

use super::error::MaintenanceError;

/// Check an attachment identifier and return it ready to paste into SQL.
///
/// The result is always a double-quoted identifier with every interior `"`
/// doubled, which is SQLite's own escaping rule, so a name such as `we"ird`
/// comes back as `"we""ird"` and names exactly that schema. Quoting is
/// unconditional: a name that happens to look like a bare identifier is quoted
/// too, so a SQL keyword cannot change the statement's meaning.
///
/// # Errors
///
/// Returns [`MaintenanceError::InvalidSchemaName`] when `name` is empty or
/// contains a NUL byte. Both are refusals made here, before any statement
/// reaches the driver. SQLite itself would accept an empty schema name, and it
/// really does address a distinct database; it is refused anyway because an
/// empty identifier is far more often an unset configuration value than an
/// intent, and it cannot be written in ordinary SQL without this same quoting
/// dance. Everything else SQLite dislikes — `main`, `temp`, a name already in
/// use — is left to SQLite, so the caller gets its message and its result code.
pub(super) fn quote_schema(name: &str) -> Result<String, MaintenanceError> {
  let reason = if name.is_empty() {
    Some("it is empty")
  } else if name.contains('\0') {
    Some("it contains a NUL byte")
  } else {
    None
  };
  if let Some(reason) = reason {
    return Err(MaintenanceError::InvalidSchemaName {
      name: name.to_owned(),
      reason,
    });
  }

  let mut quoted = String::with_capacity(name.len() + 2);
  quoted.push('"');
  for ch in name.chars() {
    if ch == '"' {
      quoted.push('"');
    }
    quoted.push(ch);
  }
  quoted.push('"');
  Ok(quoted)
}
