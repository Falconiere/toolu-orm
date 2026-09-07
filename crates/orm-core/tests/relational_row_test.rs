use toolu_orm_core::relational_row::{
  parse_json_array_of_arrays, parse_json_single_array, RelationDeserializer,
};

#[test]
fn parse_json_array_of_arrays_returns_vec_of_vecs() -> Result<(), Box<dyn std::error::Error>> {
  let json = r#"[[1,"hello",true],[2,"world",false]]"#;
  let rows = parse_json_array_of_arrays(json)?;
  assert_eq!(rows.len(), 2);
  let r0 = rows.first().ok_or("missing r0")?;
  let r1 = rows.get(1).ok_or("missing r1")?;
  assert_eq!(r0.len(), 3);
  assert_eq!(r0.first(), Some(&serde_json::json!(1)));
  assert_eq!(r0.get(1), Some(&serde_json::json!("hello")));
  assert_eq!(r1.first(), Some(&serde_json::json!(2)));
  Ok(())
}

#[test]
fn parse_json_array_of_arrays_handles_empty_array() -> Result<(), Box<dyn std::error::Error>> {
  let json = "[]";
  let rows = parse_json_array_of_arrays(json)?;
  assert_eq!(rows.len(), 0);
  Ok(())
}

#[test]
fn parse_json_array_of_arrays_returns_error_for_invalid_json() {
  let json = "not valid json";
  assert!(parse_json_array_of_arrays(json).is_err());
}

#[test]
fn parse_json_single_array_returns_vec_of_values() -> Result<(), Box<dyn std::error::Error>> {
  let json = r#"[42,"name",null]"#;
  let values = parse_json_single_array(json)?;
  assert_eq!(values.len(), 3);
  assert_eq!(values.first(), Some(&serde_json::json!(42)));
  assert_eq!(values.get(1), Some(&serde_json::json!("name")));
  assert!(values.get(2).is_some_and(|v| v.is_null()));
  Ok(())
}

#[test]
fn parse_json_single_array_returns_none_equivalent_for_null_string(
) -> Result<(), Box<dyn std::error::Error>> {
  let json = "null";
  let values = parse_json_single_array(json)?;
  assert_eq!(values.len(), 0);
  Ok(())
}

#[test]
fn relation_deserializer_extracts_string_value() -> Result<(), Box<dyn std::error::Error>> {
  let values = vec![
    serde_json::json!(42),
    serde_json::json!("hello"),
    serde_json::json!(true),
  ];
  let deser = RelationDeserializer::new(&values);
  let s: String = deser.get(1)?;
  assert_eq!(s, "hello");
  Ok(())
}
