//! `execute_sql` (write path): reuse across changed parameters.

use toolu_orm_core::value::Value;
use toolu_orm_query::executor::Executor;

use crate::db;
use crate::users::{User, USER_COLUMNS};

#[test]
fn execute_sql_reuses_cached_statement_across_changed_parameters(
) -> Result<(), Box<dyn std::error::Error>> {
  let conn = db::setup_db()?;
  const INSERT: &str = "INSERT INTO users (id, name, email, age) VALUES (?1, ?2, ?3, ?4)";

  let affected_first = Executor::execute_sql(
    &conn,
    INSERT,
    vec![
      Value::Text("u1".to_owned()),
      Value::Text("Ann".to_owned()),
      Value::Text("ann@example.com".to_owned()),
      Value::Integer(30),
    ],
  )?;
  let affected_second = Executor::execute_sql(
    &conn,
    INSERT,
    vec![
      Value::Text("u2".to_owned()),
      Value::Text("Bea".to_owned()),
      Value::Text("bea@example.com".to_owned()),
      Value::Integer(31),
    ],
  )?;

  assert_eq!(affected_first, 1);
  assert_eq!(affected_second, 1);

  let rows: Vec<User> = Executor::query_map(
    &conn,
    &format!("SELECT {} FROM users ORDER BY id", USER_COLUMNS.join(", ")),
    vec![],
  )?;
  match rows.as_slice() {
    [ann, bea] => {
      assert_eq!(ann.name, "Ann");
      assert_eq!(bea.name, "Bea");
    },
    other => return Err(format!("expected exactly two rows, got: {other:?}").into()),
  }
  Ok(())
}
