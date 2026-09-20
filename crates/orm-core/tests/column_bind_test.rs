//! Column markers retag binds. `true` on an integer column stays `1`; on a
//! boolean column it becomes [`Value::Boolean`]. SQLite still stores that tag
//! as `0`/`1`.

use toolu_orm_core::column::{Boolean, Integer, Jsonb, Numeric, Timestamp, Uuid};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::Expr;
use toolu_orm_core::query_column::{Column, CommonOps, NumericOps};
use toolu_orm_core::value::{JsonStorage, Value};

fn params(expr: &Expr) -> Vec<Value> {
  expr.to_sql_fragment_for(1, Dialect::Sqlite).1
}

#[test]
fn boolean_column_tags_bool_and_sqlite_stores_integer() {
  let active: Column<Boolean> = Column::new("flags", "active");
  assert_eq!(params(&active.eq(true)), vec![Value::Boolean(true)]);
  assert_eq!(params(&active.eq(false)), vec![Value::Boolean(false)]);
  assert_eq!(Value::Boolean(true).sqlite_stored(), Value::Integer(1));
  assert_eq!(Value::Boolean(false).sqlite_stored(), Value::Integer(0));
}

#[test]
fn integer_column_keeps_bool_as_integer() {
  let n: Column<Integer> = Column::new("flags", "n");
  assert_eq!(params(&n.eq(true)), vec![Value::Integer(1)]);
  assert_eq!(params(&n.eq(false)), vec![Value::Integer(0)]);
}

#[test]
fn timestamp_column_tags_epoch_and_rfc3339() {
  let seen: Column<Timestamp> = Column::new("events", "seen");
  assert_eq!(params(&seen.eq(0_i64)), vec![Value::TimestampEpoch(0)]);
  assert_eq!(
    params(&seen.gt("1970-01-01T00:00:00Z")),
    vec![Value::TimestampText("1970-01-01T00:00:00Z".to_owned())]
  );
  assert_eq!(Value::TimestampEpoch(0).sqlite_stored(), Value::Integer(0));
  assert_eq!(
    Value::TimestampText("1970-01-01T00:00:00Z".to_owned()).sqlite_stored(),
    Value::Text("1970-01-01T00:00:00Z".to_owned())
  );
}

#[test]
fn uuid_jsonb_and_numeric_columns_tag_their_payloads() {
  let token: Column<Uuid> = Column::new("rows", "token");
  let meta: Column<Jsonb> = Column::new("rows", "meta");
  let amount: Column<Numeric> = Column::new("rows", "amount");
  assert_eq!(
    params(&token.eq("11111111-1111-1111-1111-111111111111")),
    vec![Value::Uuid(
      "11111111-1111-1111-1111-111111111111".to_owned()
    )]
  );
  assert_eq!(
    params(&meta.eq(r#"{"ok":true}"#)),
    vec![Value::Json {
      text: r#"{"ok":true}"#.to_owned(),
      storage: JsonStorage::Jsonb,
    }]
  );
  assert_eq!(
    params(&amount.eq("12.50")),
    vec![Value::Numeric("12.50".to_owned())]
  );
  assert_eq!(
    Value::Numeric("12.50".to_owned()).sqlite_stored(),
    Value::Text("12.50".to_owned())
  );
}

#[cfg(feature = "libsql")]
#[test]
fn boolean_reaches_libsql_as_integer() {
  let stored: toolu_orm_core::libsql::Value = Value::Boolean(false).into();
  assert!(matches!(stored, toolu_orm_core::libsql::Value::Integer(0)));
}

#[cfg(feature = "postgres")]
mod postgres_encode {
  use toolu_orm_core::error::DbCoreError;
  use toolu_orm_core::value::{to_pg_params, Value};

  #[test]
  fn rejects_a_uuid_timestamp_json_and_numeric_that_cannot_be_encoded() {
    assert!(to_pg_params(&[Value::Uuid("not-a-uuid".to_owned())]).is_err());
    assert!(to_pg_params(&[Value::TimestampText("yesterday".to_owned())]).is_err());
    assert!(to_pg_params(&[Value::Json {
      text: "{".to_owned(),
      storage: toolu_orm_core::value::JsonStorage::Json,
    }])
    .is_err());
    assert!(to_pg_params(&[Value::Numeric("nope".to_owned())]).is_err());
  }

  #[test]
  fn accepts_the_tagged_payloads_the_live_suite_binds() -> Result<(), DbCoreError> {
    to_pg_params(&[
      Value::Boolean(true),
      Value::TimestampEpoch(0),
      Value::TimestampText("1970-01-01T00:00:00Z".to_owned()),
      Value::Uuid("11111111-1111-1111-1111-111111111111".to_owned()),
      Value::Json {
        text: r#"{"ok":true}"#.to_owned(),
        storage: toolu_orm_core::value::JsonStorage::Jsonb,
      },
      Value::Numeric("12.50".to_owned()),
    ])?;
    Ok(())
  }
}
