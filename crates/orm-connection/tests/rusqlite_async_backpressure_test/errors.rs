//! Failures still travel the way they did before the gate, and none of them
//! leaves the gate shut.

use std::sync::Arc;
use std::time::Duration;

use toolu_orm_connection::{DbConnection, DbError};
use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::row::FromRow;

use crate::support;

/// A later call must not wait on a permit the failed one kept.
const RECOVERY_BUDGET: Duration = Duration::from_secs(5);

/// Asks for an integer from the `label` TEXT column, which rusqlite refuses.
struct WrongTypeRow;

impl FromRow for WrongTypeRow {
  const REQUIRED_COLUMNS: &'static [&'static str] = &["label"];

  fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, DbCoreError> {
    let _mismatched: i64 = row
      .get(0)
      .map_err(|e| DbCoreError::RowMapping(e.to_string()))?;
    Ok(Self)
  }
}

/// Unwinds on purpose. `panic!` is denied workspace-wide and exempts only test
/// functions, not this `FromRow` impl; removing from an empty vector is a real
/// panic, which is all this needs.
fn unwind_deliberately() -> DbCoreError {
  let mut empty: Vec<DbCoreError> = Vec::new();
  empty.remove(0)
}

struct PanicRow;

impl FromRow for PanicRow {
  const REQUIRED_COLUMNS: &'static [&'static str] = &["label"];

  fn from_row(_row: &rusqlite::Row<'_>) -> Result<Self, DbCoreError> {
    Err(unwind_deliberately())
  }
}

#[test]
fn a_decode_failure_is_still_a_row_mapping_error() -> Result<(), Box<dyn std::error::Error>> {
  let runtime = support::runtime(2)?;
  runtime.block_on(async {
    let conn = support::seeded_connection()?;
    support::insert_label(&conn, "not-an-integer")?;

    let failed = DbConnection::query_map::<WrongTypeRow>(&conn, support::SELECT_LABEL, vec![])
      .await
      .err()
      .ok_or("the mismatched column type should have failed the query")?;
    assert!(
      matches!(&failed, DbError::RowMapping(message) if message.contains("Invalid column type")),
      "expected the FromRow message in a row-mapping error, got {failed:?}"
    );

    // The permit came back, so the connection still answers.
    let rows: Vec<crate::rusqlite_rows::LabelRow> =
      DbConnection::query_map(&conn, support::SELECT_LABEL, vec![]).await?;
    assert_eq!(rows.len(), 1);
    Ok(())
  })
}

/// The permit is dropped while the blocking task unwinds, so the gate reopens
/// and the next caller gets the poisoning report instead of waiting forever.
#[test]
fn a_panicking_decode_releases_the_permit() -> Result<(), Box<dyn std::error::Error>> {
  let runtime = support::runtime(2)?;
  runtime.block_on(async {
    let conn = Arc::new(support::seeded_connection()?);
    support::insert_label(&conn, "held")?;

    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_info| {}));
    let panicked = DbConnection::query_map::<PanicRow>(&*conn, support::SELECT_LABEL, vec![]).await;
    std::panic::set_hook(previous_hook);

    let join_error = panicked
      .err()
      .ok_or("the panicking decode should have failed the query")?;
    assert!(
      matches!(&join_error, DbError::Connection(message) if message.contains("blocking task did not complete")),
      "expected the join failure to be reported as a connection error, got {join_error:?}"
    );

    let next = tokio::time::timeout(
      RECOVERY_BUDGET,
      DbConnection::execute_batch(&*conn, "SELECT 1"),
    )
    .await
    .map_err(|elapsed| format!("the next call never got a permit back: {elapsed}"))?;
    let reported = next
      .err()
      .ok_or("the poisoned connection should have been refused")?;
    assert!(
      matches!(&reported, DbError::Connection(message) if message.contains("poisoned")),
      "expected the poisoning report once the gate reopened, got {reported:?}"
    );
    Ok(())
  })
}
