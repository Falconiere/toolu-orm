//! `execute_sql` (write path): reuse across changed parameters.

use toolu_orm_connection::DbConnectionBlocking;

use crate::blocking_conn::open_in_memory;
use crate::rusqlite_rows::LabelRow;
use crate::support::{create_items, insert_item};

#[test]
fn execute_sql_reuses_cached_statement_across_changed_parameters()
-> Result<(), Box<dyn std::error::Error>> {
  let conn = open_in_memory()?;
  create_items(&conn)?;

  let affected_first = insert_item(&conn, 1, "alpha")?;
  let affected_second = insert_item(&conn, 2, "beta")?;

  assert_eq!(affected_first, 1);
  assert_eq!(affected_second, 1);

  let rows: Vec<LabelRow> =
    DbConnectionBlocking::query_map(&conn, "SELECT label FROM items ORDER BY id", vec![])?;
  match rows.as_slice() {
    [alpha, beta] => {
      assert_eq!(alpha.label, "alpha");
      assert_eq!(beta.label, "beta");
    },
    other => return Err(format!("expected exactly two rows, got: {other:?}").into()),
  }
  Ok(())
}
