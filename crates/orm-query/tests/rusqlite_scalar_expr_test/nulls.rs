//! AC-3: `COALESCE` in a projection and in an `ORDER BY` term.

use toolu_orm_query::select::SelectBuilder;

use crate::db::{self, ids, Memory};
use crate::seed::ID;
use crate::support::{all_memories, effective_time, Effective, TestResult};

fn effective(id: &str, stamp: &str) -> Effective {
  Effective {
    id: id.to_owned(),
    effective: stamp.to_owned(),
  }
}

#[test]
fn coalesce_falls_back_to_created_at_when_last_accessed_is_null() -> TestResult {
  let conn = db::setup_db()?;

  let rows: Vec<Effective> = SelectBuilder::new("memories")
    .columns_raw(&["id"])
    .column_scalar(effective_time()?, "effective")
    .order_by(ID.asc())
    .fetch_all(&conn)?;

  assert_eq!(
    rows,
    vec![
      effective("m1", "2026-09-18T10:00:00Z"),
      effective("m2", "2026-09-19T08:00:00Z"),
      effective("m3", "2026-09-18T09:59:59.999999Z"),
      effective("m4", "2026-09-16T08:00:00Z"),
    ]
  );
  Ok(())
}

#[test]
fn ordering_by_coalesce_ranks_rows_by_their_effective_time() -> TestResult {
  let conn = db::setup_db()?;

  let rows: Vec<Memory> = all_memories()
    .order_by(effective_time()?.desc())
    .fetch_all(&conn)?;

  assert_eq!(ids(&rows), vec!["m2", "m1", "m3", "m4"]);
  Ok(())
}
