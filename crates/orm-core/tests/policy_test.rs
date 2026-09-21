//! `PolicyDef` / `RowSecurity`: the builder, the SQL keywords, and the JSON
//! shape a snapshot records — defaults omitted, nothing else.

use toolu_orm_core::policy::{PolicyCommand, PolicyDef, PolicyKind, RowSecurity};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn builder_sets_every_clause() {
  let policy = PolicyDef::new("tenant_isolation")
    .kind(PolicyKind::Restrictive)
    .command(PolicyCommand::Update)
    .role("app_user")
    .role("app_admin")
    .using("tenant_id = 1")
    .with_check("tenant_id = 1");
  assert_eq!(policy.name, "tenant_isolation");
  assert_eq!(policy.kind, PolicyKind::Restrictive);
  assert_eq!(policy.command, PolicyCommand::Update);
  assert_eq!(policy.roles, vec!["app_user", "app_admin"]);
  assert_eq!(policy.using.as_deref(), Some("tenant_id = 1"));
  assert_eq!(policy.with_check.as_deref(), Some("tenant_id = 1"));
}

#[test]
fn defaults_are_permissive_for_all_to_public() {
  let policy = PolicyDef::new("p");
  assert!(policy.kind.is_permissive());
  assert!(policy.command.is_all());
  assert!(policy.roles.is_empty());
  assert_eq!(policy.using, None);
  assert_eq!(policy.with_check, None);
}

#[test]
fn command_keywords_and_clause_rules() {
  assert_eq!(PolicyCommand::All.as_sql(), "ALL");
  assert_eq!(PolicyCommand::Select.as_sql(), "SELECT");
  assert_eq!(PolicyCommand::Insert.as_sql(), "INSERT");
  assert_eq!(PolicyCommand::Update.as_sql(), "UPDATE");
  assert_eq!(PolicyCommand::Delete.as_sql(), "DELETE");
  assert!(!PolicyCommand::Insert.accepts_using());
  assert!(PolicyCommand::Update.accepts_using());
  assert!(!PolicyCommand::Select.accepts_with_check());
  assert!(!PolicyCommand::Delete.accepts_with_check());
  assert!(PolicyCommand::All.accepts_with_check());
  assert_eq!(PolicyKind::Permissive.as_sql(), "PERMISSIVE");
  assert_eq!(PolicyKind::Restrictive.as_sql(), "RESTRICTIVE");
}

#[test]
fn row_security_constructors() {
  assert!(!RowSecurity::enabled().force);
  assert!(RowSecurity::forced().force);
  let with_policy = RowSecurity::enabled().policy(PolicyDef::new("p").using("true"));
  assert_eq!(with_policy.policies.len(), 1);
}

#[test]
fn minimal_policy_serializes_to_name_and_expression_only() -> TestResult {
  let json = serde_json::to_value(PolicyDef::new("p").using("true"))?;
  assert_eq!(json, serde_json::json!({ "name": "p", "using": "true" }));
  Ok(())
}

#[test]
fn full_policy_round_trips_through_json() -> TestResult {
  let policy = PolicyDef::new("p")
    .kind(PolicyKind::Restrictive)
    .command(PolicyCommand::Insert)
    .role("writer")
    .with_check("owner = current_user");
  let json = serde_json::to_value(&policy)?;
  assert_eq!(
    json,
    serde_json::json!({
      "name": "p",
      "kind": "restrictive",
      "command": "insert",
      "roles": ["writer"],
      "with_check": "owner = current_user"
    })
  );
  let back: PolicyDef = serde_json::from_value(json)?;
  assert_eq!(back, policy);
  Ok(())
}

#[test]
fn row_security_json_omits_defaults() -> TestResult {
  assert_eq!(
    serde_json::to_value(RowSecurity::enabled())?,
    serde_json::json!({})
  );
  assert_eq!(
    serde_json::to_value(RowSecurity::forced())?,
    serde_json::json!({ "force": true })
  );
  let parsed: RowSecurity = serde_json::from_str("{}")?;
  assert_eq!(parsed, RowSecurity::enabled());
  Ok(())
}
