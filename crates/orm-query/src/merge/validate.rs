//! Preflight source shape and key validation, without querying the target.

use super::{Matched, MergeBuilder, NotMatched};
use std::collections::HashSet;
use toolu_orm_connection::{require_capabilities, Capability, DbError};
use toolu_orm_core::{alias::quote_ident, dialect::Dialect, value::Value};

fn invalid(message: &str) -> DbError {
  DbError::Query(format!("invalid merge: {message}"))
}

impl MergeBuilder {
  /// Validate capability, policies, source dimensions and unambiguous keys.
  ///
  /// # Errors
  /// Returns a capability refusal first, otherwise a merge-specific query error.
  pub fn validate_for(&self, dialect: Dialect) -> Result<(), DbError> {
    require_capabilities(dialect, &[Capability::KeyMerge])?;
    let mut expected = self.target_sql();
    if let Some(alias) = self.table.alias() {
      expected.push_str(&format!(" AS {}", quote_ident(alias)));
    }
    if self.table.to_sql_fragment_for(1, dialect) != (expected, Vec::new()) {
      return Err(invalid("target must be a relation"));
    }
    if self.columns.is_empty() || self.keys.is_empty() || self.rows.is_empty() {
      return Err(invalid("columns, keys and source rows must be nonempty"));
    }
    for names in [&self.columns, &self.keys] {
      let unique: HashSet<_> = names.iter().collect();
      if unique.len() != names.len() || names.iter().any(String::is_empty) {
        return Err(invalid(
          "column and key names must be nonempty and distinct",
        ));
      }
    }
    if self.keys.iter().any(|key| !self.columns.contains(key)) {
      return Err(invalid("each key must be a supplied column"));
    }
    if self.matched.is_none() || self.not_matched.is_none() {
      return Err(invalid("both matched and unmatched policies are required"));
    }
    if self.matched == Some(Matched::DoNothing) && self.not_matched == Some(NotMatched::DoNothing) {
      return Err(invalid("at least one policy must write"));
    }
    if self.matched == Some(Matched::Update) && self.columns.len() == self.keys.len() {
      return Err(invalid("update requires a non-key column"));
    }
    self.validate_keys()
  }

  fn validate_keys(&self) -> Result<(), DbError> {
    let positions: Vec<_> = self
      .columns
      .iter()
      .enumerate()
      .filter_map(|(i, name)| self.keys.contains(name).then_some(i))
      .collect();
    let mut seen: HashSet<Vec<Key<'_>>> = HashSet::new();
    let mut kinds = None;
    for row in &self.rows {
      if row.len() != self.columns.len() {
        return Err(invalid("row width differs from declared columns"));
      }
      let mut tuple = Vec::new();
      for position in &positions {
        let key = match row.get(*position) {
          Some(Value::Integer(n)) => Key::Integer(*n),
          Some(Value::Text(s)) => Key::Text(s),
          Some(Value::Boolean(b)) => Key::Boolean(*b),
          _ => {
            return Err(invalid(
              "keys require non-null integer, text or boolean values",
            ))
          },
        };
        tuple.push(key);
      }
      let row_kinds: Vec<_> = tuple.iter().map(std::mem::discriminant).collect();
      if kinds.as_ref().is_some_and(|first| first != &row_kinds) {
        return Err(invalid("key value types must be consistent across rows"));
      }
      kinds = Some(row_kinds);
      if !seen.insert(tuple) {
        return Err(invalid("repeated source key"));
      }
    }
    Ok(())
  }

  pub(super) fn target_sql(&self) -> String {
    let table = quote_ident(self.table.table());
    self.table.database().map_or(table.clone(), |schema| {
      format!("{}.{table}", quote_ident(schema))
    })
  }
}

#[derive(PartialEq, Eq, Hash)]
enum Key<'a> {
  Integer(i64),
  Text(&'a str),
  Boolean(bool),
}
