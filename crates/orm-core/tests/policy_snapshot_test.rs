//! Row security in the snapshot: written only when declared, read back
//! unchanged, and absent from (and harmless in) a snapshot that predates it.

use toolu_orm_core::column::{ColumnDef, ColumnType};
use toolu_orm_core::policy::{PolicyCommand, PolicyDef, RowSecurity};
use toolu_orm_core::schema::SchemaRegistry;
use toolu_orm_core::snapshot::Snapshot;
use toolu_orm_core::table::{TableDef, TableKind};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn docs(security: Option<RowSecurity>) -> TableDef {
  TableDef {
    name: "docs".to_owned(),
    columns: vec![ColumnDef {
      name: "id".to_owned(),
      column_type: ColumnType::Text,
      primary_key: true,
      not_null: true,
      default: None,
      unique: false,
      references: None,
      on_delete: None,
      on_update: None,
      check: None,
      unindexed: false,
      autoincrement: false,
    }],
    indexes: vec![],
    primary_key: vec![],
    strict: false,
    kind: TableKind::Ordinary,
    fts5_sync: None,
    row_security: security,
  }
}

#[test]
fn undeclared_row_security_is_absent_from_the_json() -> TestResult {
  let snap = Snapshot::from_registry(&SchemaRegistry::from_tables(vec![docs(None)]));
  let json = serde_json::to_value(&snap)?;
  let docs = json
    .get("tables")
    .and_then(|t| t.get("docs"))
    .ok_or("docs missing from json")?;
  assert_eq!(docs.get("row_security"), None);
  Ok(())
}

#[test]
fn declared_row_security_round_trips() -> TestResult {
  let security = RowSecurity::forced().policy(
    PolicyDef::new("tenant_isolation")
      .command(PolicyCommand::Select)
      .role("app_user")
      .using("tenant_id = 1"),
  );
  let table = docs(Some(security.clone()));
  let snap = Snapshot::from_registry(&SchemaRegistry::from_tables(vec![table.clone()]));
  let json = serde_json::to_string_pretty(&snap)?;
  assert!(json.contains("\"row_security\""), "json: {json}");
  assert!(json.contains("\"force\": true"), "json: {json}");

  let parsed: Snapshot = serde_json::from_str(&json)?;
  let stored = parsed
    .tables
    .get("docs")
    .and_then(|t| t.row_security.as_ref())
    .ok_or("row_security missing after round trip")?;
  assert_eq!(stored, &security);
  let registry = parsed.to_registry();
  assert_eq!(registry.find_table("docs").ok_or("docs missing")?, &table);
  Ok(())
}

#[test]
fn snapshot_without_the_field_reads_as_undeclared() -> TestResult {
  let json = r#"{
    "version": 1,
    "tables": {
      "docs": {
        "columns": {
          "id": {
            "name": "id", "column_type": "Text", "primary_key": true, "not_null": true, "unique": false
          }
        },
        "indexes": {},
        "foreign_keys": {},
        "check_constraints": {},
        "strict": false
      }
    }
  }"#;
  let parsed: Snapshot = serde_json::from_str(json)?;
  assert_eq!(
    parsed
      .tables
      .get("docs")
      .and_then(|t| t.row_security.clone()),
    None
  );
  Ok(())
}
