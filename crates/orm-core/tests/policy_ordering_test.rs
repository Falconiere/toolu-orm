//! Row-security operations take the drop and create tiers: a policy drop
//! runs with the other drops, the flag change and policy creates with the
//! other creates, and the flags come before the policies they govern.

use toolu_orm_core::diff::Operation;
use toolu_orm_core::ordering::order_operations;
use toolu_orm_core::policy::PolicyDef;

#[test]
fn drop_policy_before_add_column_and_creates_after() {
  let ops = vec![
    Operation::CreatePolicy {
      table: "docs".to_owned(),
      policy: PolicyDef::new("p").using("true"),
    },
    Operation::AlterRowLevelSecurity {
      table: "docs".to_owned(),
      enabled: true,
      force: false,
    },
    Operation::AddColumn {
      table: "docs".to_owned(),
      column: toolu_orm_core::column::ColumnDef {
        name: "c".to_owned(),
        column_type: toolu_orm_core::column::ColumnType::Text,
        primary_key: false,
        not_null: false,
        default: None,
        unique: false,
        references: None,
        on_delete: None,
        on_update: None,
        check: None,
        unindexed: false,
        autoincrement: false,
      },
    },
    Operation::DropPolicy {
      table: "docs".to_owned(),
      name: "old".to_owned(),
    },
  ];
  let ordered = order_operations(ops);
  let kinds: Vec<&str> = ordered.iter().map(kind).collect();
  assert_eq!(
    kinds,
    vec!["drop_policy", "add_column", "create_policy", "alter_rls"]
  );
}

fn kind(op: &Operation) -> &'static str {
  if matches!(op, Operation::DropPolicy { .. }) {
    "drop_policy"
  } else if matches!(op, Operation::AddColumn { .. }) {
    "add_column"
  } else if matches!(op, Operation::AlterRowLevelSecurity { .. }) {
    "alter_rls"
  } else if matches!(op, Operation::CreatePolicy { .. }) {
    "create_policy"
  } else {
    "other"
  }
}

#[test]
fn diff_order_keeps_flags_before_policies_within_the_create_tier() {
  let ops = vec![
    Operation::AlterRowLevelSecurity {
      table: "docs".to_owned(),
      enabled: true,
      force: false,
    },
    Operation::CreatePolicy {
      table: "docs".to_owned(),
      policy: PolicyDef::new("p").using("true"),
    },
  ];
  let ordered = order_operations(ops);
  let kinds: Vec<&str> = ordered.iter().map(kind).collect();
  assert_eq!(kinds, vec!["alter_rls", "create_policy"]);
}
