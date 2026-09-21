//! `#[policy(...)]` and `#[table(rls = …)]` land on `TableDef.row_security`.
//! That this file compiles also proves `#[policy]` is stripped from the
//! re-emitted struct: it is not a real attribute.

use toolu_orm_core::column::{Integer, Text};
use toolu_orm_core::policy::{PolicyCommand, PolicyDef, PolicyKind, RowSecurity};
use toolu_orm_core::table::TableSchema;
use toolu_orm_macros::table;

#[table(name = "docs")]
#[policy(
  "tenant_isolation",
  using = "tenant_id = current_setting('app.tenant_id')::int"
)]
#[policy(
  "writers_only",
  for = update,
  as = restrictive,
  to = ["app_writer", "app_admin"],
  using = "owner = current_user",
  with_check = "owner = current_user"
)]
#[policy("insert_own", for = insert, to = "app_writer", with_check = "owner = current_user")]
pub struct Docs {
  #[column(primary_key)]
  pub id: Text,
  #[column(not_null)]
  pub tenant_id: Integer,
  pub owner: Text,
}

#[table(name = "locked", rls = "force")]
pub struct Locked {
  #[column(primary_key)]
  pub id: Text,
}

#[table(name = "open_table")]
pub struct Open {
  #[column(primary_key)]
  pub id: Text,
}

#[test]
fn policies_imply_enabled_row_security() -> Result<(), Box<dyn std::error::Error>> {
  let security = Docs::table_def()
    .row_security
    .ok_or("policies should enable row security")?;
  assert!(!security.force);
  assert_eq!(
    security.policies,
    vec![
      PolicyDef::new("tenant_isolation").using("tenant_id = current_setting('app.tenant_id')::int"),
      PolicyDef::new("writers_only")
        .kind(PolicyKind::Restrictive)
        .command(PolicyCommand::Update)
        .role("app_writer")
        .role("app_admin")
        .using("owner = current_user")
        .with_check("owner = current_user"),
      PolicyDef::new("insert_own")
        .command(PolicyCommand::Insert)
        .role("app_writer")
        .with_check("owner = current_user"),
    ]
  );
  Ok(())
}

#[test]
fn rls_force_without_policies_is_forced_default_deny() {
  assert_eq!(
    Locked::table_def().row_security,
    Some(RowSecurity::forced())
  );
}

#[test]
fn table_without_the_attributes_declares_none() {
  assert_eq!(Open::table_def().row_security, None);
}
