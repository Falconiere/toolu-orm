//! Identifier validation, config quoting, and weight-array rendering for
//! Postgres FTS calls.

use crate::dialect::Dialect;
use crate::error::DbCoreError;

/// The three Postgres tsquery constructors this surface exposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TsQueryFn {
  ToTsQuery,
  PlainToTsQuery,
  WebsearchToTsQuery,
}

impl TsQueryFn {
  pub(crate) const fn as_sql(self) -> &'static str {
    match self {
      Self::ToTsQuery => "to_tsquery",
      Self::PlainToTsQuery => "plainto_tsquery",
      Self::WebsearchToTsQuery => "websearch_to_tsquery",
    }
  }
}

/// Refuses anything but Postgres.
///
/// Postgres full-text (`@@` / `to_tsquery` / `ts_rank`) is a different model
/// from SQLite FTS5. Rejecting here means no SQLite SQL containing these forms
/// can be built from this module.
pub(crate) fn require_postgres(function: &str, dialect: Dialect) -> Result<(), DbCoreError> {
  match dialect {
    Dialect::Postgres => Ok(()),
    Dialect::Sqlite => Err(DbCoreError::PgFtsUnsupportedDialect {
      function: function.to_owned(),
      dialect: dialect.as_str(),
    }),
  }
}

fn invalid(function: &str, reason: impl Into<String>) -> DbCoreError {
  DbCoreError::PgFtsInvalidArgument {
    function: function.to_owned(),
    reason: reason.into(),
  }
}

/// A text-search config name as a single-quoted literal (`'english'`).
///
/// Validated as a plain ASCII identifier — never escaped as free text — so a
/// quote, space, or comment cannot reach the SQL.
pub(crate) fn quoted_config(function: &str, config: &str) -> Result<String, DbCoreError> {
  require_plain_ident(function, "config", config)?;
  Ok(format!("'{config}'"))
}

/// Validates `name` as `[A-Za-z_][A-Za-z0-9_]*`.
pub(crate) fn require_plain_ident(
  function: &str,
  what: &str,
  name: &str,
) -> Result<(), DbCoreError> {
  let mut chars = name.chars();
  let first = chars
    .next()
    .ok_or_else(|| invalid(function, format!("the {what} name is empty")))?;
  if !(first.is_ascii_alphabetic() || first == '_') {
    return Err(invalid(
      function,
      format!("{what} name {name:?} must start with an ASCII letter or underscore"),
    ));
  }
  if !chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
    return Err(invalid(
      function,
      format!("{what} name {name:?} must contain only ASCII letters, digits and underscores"),
    ));
  }
  Ok(())
}

/// Dollar-quote `text` so it embeds in SQL without escape-sequence hazards.
///
/// Chooses a tag that does not appear in `text`, so `$tag$…$tag$` is opaque
/// even when `standard_conforming_strings` is off.
pub(crate) fn dollar_quote(text: &str) -> String {
  let mut n = 0_u32;
  loop {
    let tag = if n == 0 {
      "q".to_owned()
    } else {
      format!("q{n}")
    };
    let open = format!("${tag}$");
    if !text.contains(&open) {
      return format!("{open}{text}{open}");
    }
    n = n.saturating_add(1);
  }
}

/// Four `ts_rank` weights (D, C, B, A) as `'{d,c,b,a}'::real[]`.
///
/// # Errors
///
/// [`DbCoreError::PgFtsInvalidArgument`] when any weight is NaN, infinite, or
/// negative.
pub(crate) fn weights_literal(function: &str, weights: &[f32; 4]) -> Result<String, DbCoreError> {
  let mut parts = Vec::with_capacity(4);
  for (index, weight) in weights.iter().enumerate() {
    validate_weight(function, index, *weight)?;
    parts.push(format!("{weight:?}"));
  }
  Ok(format!("'{{{}}}'::real[]", parts.join(",")))
}

fn validate_weight(function: &str, index: usize, weight: f32) -> Result<(), DbCoreError> {
  let reason = if weight.is_nan() {
    "is NaN"
  } else if weight.is_infinite() {
    "is infinite"
  } else if weight < 0.0 {
    "is negative"
  } else {
    return Ok(());
  };
  Err(invalid(
    function,
    format!("weight #{index} ({weight}) {reason}; weights must be finite and >= 0"),
  ))
}
