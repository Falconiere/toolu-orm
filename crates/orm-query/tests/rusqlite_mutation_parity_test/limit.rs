//! Bundled rusqlite does not enable `SQLITE_ENABLE_UPDATE_DELETE_LIMIT`.

use super::support::{setup, TestResult};

#[test]
fn bundled_sqlite_rejects_order_by_and_limit_on_update_and_delete() -> TestResult {
  let conn = setup()?;
  let enabled: i64 = conn.query_row(
    "SELECT sqlite_compileoption_used('ENABLE_UPDATE_DELETE_LIMIT')",
    [],
    |row| row.get(0),
  )?;
  assert_eq!(
    enabled, 0,
    "this build has no UPDATE/DELETE ORDER BY or LIMIT"
  );

  let update_result = conn.execute("UPDATE products SET name = 'c' ORDER BY sku LIMIT 1", []);
  let Err(update_err) = update_result else {
    return Err("ORDER BY on UPDATE is a syntax error without the compile option".into());
  };
  let delete_result = conn.execute("DELETE FROM products ORDER BY sku LIMIT 1", []);
  let Err(delete_err) = delete_result else {
    return Err("ORDER BY on DELETE is a syntax error without the compile option".into());
  };
  assert!(
    update_err.to_string().contains("syntax"),
    "unexpected update error: {update_err}"
  );
  assert!(
    delete_err.to_string().contains("syntax"),
    "unexpected delete error: {delete_err}"
  );
  Ok(())
}
