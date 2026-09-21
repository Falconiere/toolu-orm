//! Validating row-security declarations, and diffing them between two
//! schema versions.
//!
//! A table's row security is diffed as one unit: the two flags become one
//! [`Operation::AlterRowLevelSecurity`] whenever either changed, and each
//! policy is keyed by name — added ones are created, removed ones dropped,
//! and a changed one is dropped and created again, the same shape the index
//! and CHECK diffs use.

use std::collections::BTreeMap;

use crate::error::DbCoreError;
use crate::policy::{PolicyDef, RowSecurity};
use crate::schema::SchemaRegistry;
use crate::snapshot::Snapshot;
use crate::table::TableDef;

use super::operation::Operation;

/// Rejects every declaration Postgres would refuse, before any operation is
/// produced — so a schema with a bad policy writes no migration at all.
///
/// # Errors
///
/// [`DbCoreError::PolicyInvalid`] naming the table, the policy and the reason.
pub(crate) fn validate(schema: &SchemaRegistry) -> Result<(), DbCoreError> {
  for table in schema.tables() {
    let Some(security) = &table.row_security else {
      continue;
    };
    let mut seen: Vec<&str> = Vec::new();
    for policy in &security.policies {
      if let Some(reason) = invalid_reason(table, policy, &seen) {
        return Err(DbCoreError::PolicyInvalid {
          table: table.name.clone(),
          policy: policy.name.clone(),
          reason,
        });
      }
      seen.push(&policy.name);
    }
    if security.policies.is_empty() && table.is_virtual() {
      return Err(DbCoreError::PolicyInvalid {
        table: table.name.clone(),
        policy: String::new(),
        reason: "a virtual table cannot carry row security".to_owned(),
      });
    }
  }
  Ok(())
}

/// Why Postgres would reject this policy, or `None` when it would not.
fn invalid_reason(table: &TableDef, policy: &PolicyDef, seen: &[&str]) -> Option<String> {
  if table.is_virtual() {
    return Some("a virtual table cannot carry row security".to_owned());
  }
  if policy.name.is_empty() {
    return Some("the policy has no name".to_owned());
  }
  if seen.contains(&policy.name.as_str()) {
    return Some("another policy on this table already has that name".to_owned());
  }
  if policy.using.is_none() && policy.with_check.is_none() {
    return Some("it has neither a USING nor a WITH CHECK expression".to_owned());
  }
  if policy.using.is_some() && !policy.command.accepts_using() {
    return Some(format!(
      "a FOR {} policy cannot have a USING expression",
      policy.command.as_sql()
    ));
  }
  if policy.with_check.is_some() && !policy.command.accepts_with_check() {
    return Some(format!(
      "a FOR {} policy cannot have a WITH CHECK expression",
      policy.command.as_sql()
    ));
  }
  None
}

/// The row-security operations that take `table_name` from `old`'s
/// declaration in `old` to the one in `new`.
pub fn diff_row_security(table_name: &str, old: &Snapshot, new: &Snapshot) -> Vec<Operation> {
  let mut ops = Vec::new();
  let old_rs = old
    .tables
    .get(table_name)
    .and_then(|t| t.row_security.as_ref());
  let new_rs = new
    .tables
    .get(table_name)
    .and_then(|t| t.row_security.as_ref());
  diff_row_security_inner(&mut ops, table_name, old_rs, new_rs);
  ops
}

pub(crate) fn diff_row_security_inner(
  ops: &mut Vec<Operation>,
  table_name: &str,
  old: Option<&RowSecurity>,
  new: Option<&RowSecurity>,
) {
  let old_flags = old.map(|rs| rs.force);
  let new_flags = new.map(|rs| rs.force);
  if old_flags != new_flags {
    ops.push(Operation::AlterRowLevelSecurity {
      table: table_name.to_owned(),
      enabled: new.is_some(),
      force: new.is_some_and(|rs| rs.force),
    });
  }

  let old_policies = by_name(old);
  let new_policies = by_name(new);
  for (name, old_policy) in &old_policies {
    let changed = new_policies.get(name).is_some_and(|p| p != old_policy);
    if !new_policies.contains_key(name) || changed {
      ops.push(Operation::DropPolicy {
        table: table_name.to_owned(),
        name: (*name).to_owned(),
      });
    }
  }
  for (name, new_policy) in &new_policies {
    let unchanged = old_policies.get(name) == Some(new_policy);
    if !unchanged {
      ops.push(Operation::CreatePolicy {
        table: table_name.to_owned(),
        policy: (*new_policy).clone(),
      });
    }
  }
}

/// The policies of a declaration by name; empty for an absent one.
fn by_name(security: Option<&RowSecurity>) -> BTreeMap<&str, &PolicyDef> {
  security
    .map(|rs| rs.policies.iter().map(|p| (p.name.as_str(), p)).collect())
    .unwrap_or_default()
}
