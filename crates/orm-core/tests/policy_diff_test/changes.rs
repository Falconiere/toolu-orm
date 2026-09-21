//! Enable with a new table, add / change / drop a policy, flip `FORCE`,
//! disable, and survive a rename.

use toolu_orm_core::diff::{diff, diff_with_resolver, Operation};
use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::policy::{PolicyCommand, PolicyDef, RowSecurity};
use toolu_orm_core::rename::RenameResolver;
use toolu_orm_core::schema::SchemaRegistry;
use toolu_orm_core::snapshot::Snapshot;

use super::support::{docs, ops_between, snapshot_of, tenant_policy};

#[test]
fn new_table_with_policy_enables_then_creates() -> Result<(), DbCoreError> {
  let new = docs(Some(RowSecurity::enabled().policy(tenant_policy())));
  let ops = diff(&Snapshot::empty(), &SchemaRegistry::from_tables(vec![new]))?;
  assert!(matches!(ops.first(), Some(Operation::CreateTable { .. })));
  assert!(matches!(
    ops.get(1),
    Some(Operation::AlterRowLevelSecurity { table, enabled: true, force: false }) if table == "docs"
  ));
  assert!(matches!(
    ops.get(2),
    Some(Operation::CreatePolicy { table, policy })
      if table == "docs" && policy.name == "tenant_isolation"
  ));
  assert_eq!(ops.len(), 3);
  Ok(())
}

#[test]
fn unchanged_declaration_diffs_to_nothing() -> Result<(), DbCoreError> {
  let security = Some(RowSecurity::forced().policy(tenant_policy()));
  let ops = ops_between(docs(security.clone()), docs(security))?;
  assert!(ops.is_empty(), "ops: {ops:?}");
  Ok(())
}

#[test]
fn adding_a_policy_creates_only_that_policy() -> Result<(), DbCoreError> {
  let old = docs(Some(RowSecurity::enabled().policy(tenant_policy())));
  let new = docs(Some(
    RowSecurity::enabled().policy(tenant_policy()).policy(
      PolicyDef::new("read_all")
        .command(PolicyCommand::Select)
        .using("true"),
    ),
  ));
  let ops = ops_between(old, new)?;
  assert_eq!(ops.len(), 1);
  assert!(matches!(
    ops.first(),
    Some(Operation::CreatePolicy { policy, .. }) if policy.name == "read_all"
  ));
  Ok(())
}

#[test]
fn changed_expression_drops_and_recreates() -> Result<(), DbCoreError> {
  let old = docs(Some(RowSecurity::enabled().policy(tenant_policy())));
  let new = docs(Some(
    RowSecurity::enabled().policy(PolicyDef::new("tenant_isolation").using("tenant_id = 1")),
  ));
  let ops = ops_between(old, new)?;
  assert!(matches!(
    ops.first(),
    Some(Operation::DropPolicy { name, .. }) if name == "tenant_isolation"
  ));
  assert!(matches!(
    ops.get(1),
    Some(Operation::CreatePolicy { policy, .. })
      if policy.using.as_deref() == Some("tenant_id = 1")
  ));
  assert_eq!(ops.len(), 2);
  Ok(())
}

#[test]
fn flipping_force_alters_flags_only() -> Result<(), DbCoreError> {
  let old = docs(Some(RowSecurity::enabled().policy(tenant_policy())));
  let new = docs(Some(RowSecurity::forced().policy(tenant_policy())));
  let ops = ops_between(old, new)?;
  assert_eq!(ops.len(), 1);
  assert!(matches!(
    ops.first(),
    Some(Operation::AlterRowLevelSecurity {
      enabled: true,
      force: true,
      ..
    })
  ));
  Ok(())
}

#[test]
fn removing_the_declaration_drops_policies_and_disables() -> Result<(), DbCoreError> {
  let old = docs(Some(RowSecurity::forced().policy(tenant_policy())));
  let ops = ops_between(old, docs(None))?;
  assert!(matches!(
    ops.first(),
    Some(Operation::AlterRowLevelSecurity {
      enabled: false,
      force: false,
      ..
    })
  ));
  assert!(matches!(
    ops.get(1),
    Some(Operation::DropPolicy { name, .. }) if name == "tenant_isolation"
  ));
  assert_eq!(ops.len(), 2);
  Ok(())
}

struct RenameDocs;

impl RenameResolver for RenameDocs {
  fn resolve_tables(&self, _added: &[String], _removed: &[String]) -> Vec<(String, String)> {
    vec![("docs".to_owned(), "documents".to_owned())]
  }

  fn resolve_columns(&self, _: &str, _: &[String], _: &[String]) -> Vec<(String, String)> {
    vec![]
  }
}

#[test]
fn renamed_table_keeps_its_policies() -> Result<(), DbCoreError> {
  let old = docs(Some(RowSecurity::enabled().policy(tenant_policy())));
  let mut new = old.clone();
  new.name = "documents".to_owned();
  let ops = diff_with_resolver(
    &snapshot_of(old),
    &SchemaRegistry::from_tables(vec![new]),
    &RenameDocs,
  )?;
  assert_eq!(ops.len(), 1, "ops: {ops:?}");
  assert!(matches!(ops.first(), Some(Operation::RenameTable { .. })));
  Ok(())
}
