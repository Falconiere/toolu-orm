use toolu_orm_core::value::Value;

#[test]
fn from_str_creates_text() {
  let v: Value = "hello".into();
  assert!(matches!(v, Value::Text(ref s) if s == "hello"));
}

#[test]
fn from_string_creates_text() {
  let v: Value = String::from("hello").into();
  assert!(matches!(v, Value::Text(ref s) if s == "hello"));
}

#[test]
fn from_i32_creates_integer() {
  let v: Value = 42i32.into();
  assert!(matches!(v, Value::Integer(42)));
}

#[test]
fn from_i64_creates_integer() {
  let v: Value = 42i64.into();
  assert!(matches!(v, Value::Integer(42)));
}

#[test]
fn from_f64_creates_real() {
  let v: Value = 2.71f64.into();
  assert!(matches!(v, Value::Real(f) if (f - 2.71).abs() < f64::EPSILON));
}

#[test]
fn from_bool_true_creates_integer_1() {
  let v: Value = true.into();
  assert!(matches!(v, Value::Integer(1)));
}

#[test]
fn from_bool_false_creates_integer_0() {
  let v: Value = false.into();
  assert!(matches!(v, Value::Integer(0)));
}

#[test]
fn from_bytes_creates_blob() {
  let v: Value = vec![1u8, 2, 3].into();
  assert!(matches!(v, Value::Blob(ref b) if *b == vec![1, 2, 3]));
}

#[test]
fn from_none_creates_null() {
  let v: Value = Option::<String>::None.into();
  assert!(matches!(v, Value::Null));
}

#[test]
fn from_some_string_creates_text() {
  let v: Value = Some("hello".to_owned()).into();
  assert!(matches!(v, Value::Text(ref s) if s == "hello"));
}

#[cfg(feature = "postgres")]
mod postgres_conversions {
  use toolu_orm_core::value::{to_pg_params, Value};

  #[test]
  fn to_pg_params_empty_input() {
    let params: Vec<Value> = vec![];
    let pg_params = to_pg_params(&params);
    assert_eq!(pg_params.len(), 0);
  }

  #[test]
  fn to_pg_params_text_value() {
    let params = vec![Value::Text("hello".to_owned())];
    let pg_params = to_pg_params(&params);
    assert_eq!(pg_params.len(), 1);
  }

  #[test]
  fn to_pg_params_integer_value() {
    let params = vec![Value::Integer(42)];
    let pg_params = to_pg_params(&params);
    assert_eq!(pg_params.len(), 1);
  }

  #[test]
  fn to_pg_params_multiple_values() {
    let params = vec![
      Value::Text("hello".to_owned()),
      Value::Integer(42),
      Value::Real(2.5),
      Value::Null,
      Value::Blob(vec![1, 2, 3]),
    ];
    let pg_params = to_pg_params(&params);
    assert_eq!(pg_params.len(), 5);
  }

  #[test]
  fn to_pg_params_null_value() {
    let params = vec![Value::Null];
    let pg_params = to_pg_params(&params);
    assert_eq!(pg_params.len(), 1);
  }

  #[test]
  fn to_pg_params_preserves_order() {
    let params = vec![
      Value::Text("first".to_owned()),
      Value::Integer(2),
      Value::Text("third".to_owned()),
    ];
    let pg_params = to_pg_params(&params);
    assert_eq!(pg_params.len(), 3);
  }
}
