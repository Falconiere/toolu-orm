//! Row-security operations rendered for Postgres, and as comments on SQLite.

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::diff::Operation;
use toolu_orm_core::policy::{PolicyCommand, PolicyDef, PolicyKind};
use toolu_orm_core::sql::generate_sql_for;

fn enable(force: bool) -> Operation {
  Operation::AlterRowLevelSecurity {
    table: "docs".to_owned(),
    enabled: true,
    force,
  }
}

#[test]
fn enable_renders_both_flags_postgres() {
  let sql = generate_sql_for(&[enable(false)], Dialect::Postgres);
  assert_eq!(
    sql,
    "ALTER TABLE \"docs\" ENABLE ROW LEVEL SECURITY;\n\n--> statement-breakpoint\n\n\
     ALTER TABLE \"docs\" NO FORCE ROW LEVEL SECURITY;"
  );
  let forced = generate_sql_for(&[enable(true)], Dialect::Postgres);
  assert!(
    forced.contains("ALTER TABLE \"docs\" FORCE ROW LEVEL SECURITY;"),
    "sql: {forced}"
  );
}

#[test]
fn disable_renders_disable_and_no_force_postgres() {
  let op = Operation::AlterRowLevelSecurity {
    table: "docs".to_owned(),
    enabled: false,
    force: false,
  };
  assert_eq!(
    generate_sql_for(&[op], Dialect::Postgres),
    "ALTER TABLE \"docs\" DISABLE ROW LEVEL SECURITY;\n\n--> statement-breakpoint\n\n\
     ALTER TABLE \"docs\" NO FORCE ROW LEVEL SECURITY;"
  );
}

#[test]
fn minimal_policy_renders_name_table_and_using() {
  let op = Operation::CreatePolicy {
    table: "docs".to_owned(),
    policy: PolicyDef::new("tenant_isolation").using("tenant_id = 1"),
  };
  assert_eq!(
    generate_sql_for(&[op], Dialect::Postgres),
    "CREATE POLICY \"tenant_isolation\" ON \"docs\" USING (tenant_id = 1);"
  );
}

#[test]
fn full_policy_renders_every_clause_in_postgres_order() {
  let op = Operation::CreatePolicy {
    table: "docs".to_owned(),
    policy: PolicyDef::new("writers_only")
      .kind(PolicyKind::Restrictive)
      .command(PolicyCommand::Update)
      .role("app_writer")
      .role("public")
      .role("current_user")
      .using("owner = current_user")
      .with_check("owner = current_user"),
  };
  assert_eq!(
    generate_sql_for(&[op], Dialect::Postgres),
    "CREATE POLICY \"writers_only\" ON \"docs\" AS RESTRICTIVE FOR UPDATE \
     TO \"app_writer\", PUBLIC, CURRENT_USER USING (owner = current_user) \
     WITH CHECK (owner = current_user);"
  );
}

#[test]
fn drop_policy_postgres() {
  let op = Operation::DropPolicy {
    table: "docs".to_owned(),
    name: "tenant_isolation".to_owned(),
  };
  assert_eq!(
    generate_sql_for(&[op], Dialect::Postgres),
    "DROP POLICY IF EXISTS \"tenant_isolation\" ON \"docs\";"
  );
}

#[test]
fn every_row_security_operation_is_a_comment_on_sqlite() {
  let ops = vec![
    enable(true),
    Operation::CreatePolicy {
      table: "docs".to_owned(),
      policy: PolicyDef::new("p").using("true"),
    },
    Operation::DropPolicy {
      table: "docs".to_owned(),
      name: "old".to_owned(),
    },
  ];
  // Ordered output: the drop takes the drop tier, the other two the create tier.
  assert_eq!(
    generate_sql_for(&ops, Dialect::Sqlite),
    "-- drop policy \"old\" on \"docs\" (Postgres only; SQLite has no row-level security)\n\n\
     --> statement-breakpoint\n\n\
     -- row level security on \"docs\" (Postgres only; SQLite has no row-level security)\n\n\
     --> statement-breakpoint\n\n\
     -- policy \"p\" on \"docs\" (Postgres only; SQLite has no row-level security)"
  );
}

#[test]
fn role_with_embedded_quote_is_doubled() {
  let op = Operation::CreatePolicy {
    table: "docs".to_owned(),
    policy: PolicyDef::new("p").role("odd\"role").using("true"),
  };
  assert_eq!(
    generate_sql_for(&[op], Dialect::Postgres),
    "CREATE POLICY \"p\" ON \"docs\" TO \"odd\"\"role\" USING (true);"
  );
}
