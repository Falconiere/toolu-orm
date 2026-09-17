//! `decode_relation_json` / `decode_relation_value`: declared binary columns
//! become JSON byte arrays, every other value passes through, and bad input is
//! reported as a `RowMapping` error naming the relation.

use serde_json::json;
use toolu_orm_query::select::{RelationColumn, RelationalSelectBuilder};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn many_builder() -> RelationalSelectBuilder {
  RelationalSelectBuilder::new("owners", &["id"]).with_many_columns(
    "files",
    "files",
    "id",
    "owner_id",
    &[RelationColumn::new("id"), RelationColumn::binary("payload")],
  )
}

fn one_builder() -> RelationalSelectBuilder {
  RelationalSelectBuilder::new("files", &["id"]).with_one_columns(
    "owner",
    "owners",
    "owner_id",
    "id",
    &[RelationColumn::new("id"), RelationColumn::binary("avatar")],
  )
}

#[test]
fn many_column_decodes_hex_to_bytes_and_leaves_other_values_alone() -> TestResult {
  let decoded =
    many_builder().decode_relation_json("files", r#"[["f1","0102FF"],["f2",null],["f3",""]]"#)?;
  assert_eq!(
    decoded,
    json!([["f1", [1, 2, 255]], ["f2", null], ["f3", []]])
  );
  Ok(())
}

#[test]
fn lowercase_hex_from_postgres_decodes_to_the_same_bytes() -> TestResult {
  let decoded =
    many_builder().decode_relation_value("files", &json!([["f1", "0102ff"], ["f2", "00"]]))?;
  assert_eq!(decoded, json!([["f1", [1, 2, 255]], ["f2", [0]]]));
  Ok(())
}

#[test]
fn empty_many_column_decodes_to_an_empty_array() -> TestResult {
  assert_eq!(
    many_builder().decode_relation_json("files", "[]")?,
    json!([])
  );
  assert_eq!(
    many_builder().decode_relation_value("files", &serde_json::Value::Null)?,
    json!([])
  );
  Ok(())
}

#[test]
fn one_column_decodes_hex_and_maps_a_missing_relation_to_null() -> TestResult {
  let decoded = one_builder().decode_relation_json("owner", r#"["u1","ff00"]"#)?;
  assert_eq!(decoded, json!(["u1", [255, 0]]));

  assert_eq!(
    one_builder().decode_relation_json("owner", "null")?,
    serde_json::Value::Null
  );
  assert_eq!(
    one_builder().decode_relation_value("owner", &serde_json::Value::Null)?,
    serde_json::Value::Null
  );
  Ok(())
}

#[test]
fn unknown_relation_field_is_reported() -> TestResult {
  let err = many_builder()
    .decode_relation_json("ghosts", "[]")
    .err()
    .ok_or("expected an error for an unknown relation")?;
  assert!(
    err.to_string().contains("unknown relation field 'ghosts'"),
    "error should name the field, got: {err}"
  );
  Ok(())
}

#[test]
fn non_hex_text_at_a_binary_index_is_reported() -> TestResult {
  for bad in [r#"[["f1","zz"]]"#, r#"[["f1","abc"]]"#] {
    let err = many_builder()
      .decode_relation_json("files", bad)
      .err()
      .ok_or("expected an error for invalid hex")?;
    let message = err.to_string();
    assert!(
      message.contains("relation 'files'") && message.contains("column 'payload'"),
      "error should name relation and column, got: {message}"
    );
    assert!(
      message.contains("not valid hex-encoded binary"),
      "error should explain the cause, got: {message}"
    );
  }
  Ok(())
}

#[test]
fn a_non_text_value_at_a_binary_index_is_reported() -> TestResult {
  let err = many_builder()
    .decode_relation_value("files", &json!([["f1", 12]]))
    .err()
    .ok_or("expected an error for a numeric binary value")?;
  assert!(
    err
      .to_string()
      .contains("column 'payload' is binary but the value is neither a string nor null"),
    "error should explain the cause, got: {err}"
  );
  Ok(())
}

#[test]
fn a_row_whose_arity_differs_from_the_projection_is_reported() -> TestResult {
  let err = many_builder()
    .decode_relation_value("files", &json!([["f1", "00", "extra"]]))
    .err()
    .ok_or("expected an error for a wrong row arity")?;
  assert!(
    err.to_string().contains("got 3 values for 2 columns"),
    "error should report both counts, got: {err}"
  );
  Ok(())
}

#[test]
fn a_malformed_json_shape_is_reported() -> TestResult {
  let err = many_builder()
    .decode_relation_value("files", &json!({"not": "an array"}))
    .err()
    .ok_or("expected an error for a non-array column")?;
  assert!(
    err.to_string().contains("relation 'files'"),
    "error should name the relation, got: {err}"
  );
  Ok(())
}

#[test]
fn a_projection_without_binary_columns_passes_every_value_through() -> TestResult {
  let plain = RelationalSelectBuilder::new("users", &["id"]).with_many(
    "posts",
    "posts",
    "id",
    "author_id",
    &["id", "title"],
  );
  let decoded = plain.decode_relation_json("posts", r#"[["p1","0102FF"],["p2","hello"]]"#)?;
  assert_eq!(
    decoded,
    json!([["p1", "0102FF"], ["p2", "hello"]]),
    "text at an undeclared index must never be treated as encoded binary"
  );
  Ok(())
}
