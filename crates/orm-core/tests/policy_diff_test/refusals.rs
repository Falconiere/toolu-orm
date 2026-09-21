//! The declarations `diff` refuses before writing anything.

use toolu_orm_core::diff::diff;
use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::policy::{PolicyCommand, PolicyDef, RowSecurity};
use toolu_orm_core::schema::SchemaRegistry;
use toolu_orm_core::snapshot::Snapshot;

use toolu_orm_core::table::TableKind;

use super::support::{docs, tenant_policy};

/// The `PolicyInvalid` reason `diff` reports for a schema declaring `policy`,
/// or `None` when it reports anything else.
fn reason_for(policy: PolicyDef) -> Option<String> {
  let new = docs(Some(RowSecurity::enabled().policy(policy)));
  let Err(DbCoreError::PolicyInvalid { reason, .. }) =
    diff(&Snapshot::empty(), &SchemaRegistry::from_tables(vec![new]))
  else {
    return None;
  };
  Some(reason)
}

#[test]
fn refuses_a_policy_without_any_expression() {
  assert_eq!(
    reason_for(PolicyDef::new("empty")).as_deref(),
    Some("it has neither a USING nor a WITH CHECK expression")
  );
}

#[test]
fn refuses_using_on_insert_and_with_check_on_select() {
  assert_eq!(
    reason_for(
      PolicyDef::new("ins")
        .command(PolicyCommand::Insert)
        .using("true")
    )
    .as_deref(),
    Some("a FOR INSERT policy cannot have a USING expression")
  );
  assert_eq!(
    reason_for(
      PolicyDef::new("sel")
        .command(PolicyCommand::Select)
        .with_check("true")
    )
    .as_deref(),
    Some("a FOR SELECT policy cannot have a WITH CHECK expression")
  );
}

#[test]
fn refuses_duplicate_policy_names() {
  let new = docs(Some(
    RowSecurity::enabled()
      .policy(tenant_policy())
      .policy(tenant_policy()),
  ));
  let err = diff(&Snapshot::empty(), &SchemaRegistry::from_tables(vec![new]));
  assert!(matches!(
    err,
    Err(DbCoreError::PolicyInvalid { policy, .. }) if policy == "tenant_isolation"
  ));
}

#[test]
fn refuses_row_security_on_a_virtual_table() {
  let reason = "a virtual table cannot carry row security";
  for security in [
    RowSecurity::enabled(),
    RowSecurity::enabled().policy(tenant_policy()),
  ] {
    let mut fts = docs(Some(security));
    fts.kind = TableKind::virtual_table("fts5", vec!["body".to_owned()]);
    let err = diff(&Snapshot::empty(), &SchemaRegistry::from_tables(vec![fts]));
    assert!(
      matches!(&err, Err(DbCoreError::PolicyInvalid { table, reason: r, .. })
        if table == "docs" && r == reason),
      "err: {err:?}"
    );
  }
}
