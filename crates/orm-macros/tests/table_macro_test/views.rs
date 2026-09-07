//! Tests for view macros (omit/pick) and serde verification.
//!
//! # Public API
//!
//! Tests: view omit, view pick, serde round-trip on view structs.

use toolu_orm_core::column::{Integer, Text, Uuid, Varchar};
use toolu_orm_macros::table;

// --- View macros ---

#[table(name = "view_items")]
#[view(ViewItemResponse, omit(internal_code))]
#[view(ViewItemSummary, pick(id, name))]
pub struct ViewItem {
  #[column(primary_key)]
  pub id: Uuid,
  #[column(not_null)]
  pub name: Varchar<255>,
  #[column(not_null)]
  pub internal_code: Text,
  pub description: Text,
  #[column(not_null, default = "0")]
  pub created_at: Integer,
}

#[test]
fn test_view_omit_generates_struct() {
  let resp = ViewItemResponse {
    id: "abc".to_owned(),
    name: "Test".to_owned(),
    description: None,
    created_at: 123,
  };
  assert_eq!(resp.id, "abc");
  // internal_code should NOT be a field on ViewItemResponse
}

#[test]
fn test_view_pick_generates_struct() {
  let summary = ViewItemSummary {
    id: "abc".to_owned(),
    name: "Test".to_owned(),
  };
  assert_eq!(summary.name, "Test");
}

// --- Extending views and serde verification ---

#[table(name = "ext_items")]
#[view(ExtItemResponse, omit(secret))]
pub struct ExtItem {
  #[column(primary_key)]
  pub id: Uuid,
  #[column(not_null)]
  pub name: Text,
  #[column(not_null)]
  pub secret: Text,
}

#[test]
fn test_view_omit_excludes_field() {
  let resp = ExtItemResponse {
    id: "abc".to_owned(),
    name: "Test".to_owned(),
  };
  assert_eq!(resp.id, "abc");
  assert_eq!(resp.name, "Test");
}

#[test]
fn test_view_struct_derives_serde() -> Result<(), Box<dyn std::error::Error>> {
  let resp = ExtItemResponse {
    id: "abc".to_owned(),
    name: "Test".to_owned(),
  };
  let json = serde_json::to_string(&resp)?;
  assert!(json.contains("abc"));
  let parsed: ExtItemResponse = serde_json::from_str(&json)?;
  assert_eq!(parsed.id, "abc");
  Ok(())
}
