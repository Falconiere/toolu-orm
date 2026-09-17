//! Issue #90's reproduction, as a regression test.
//!
//! Two blocking threads is the whole budget: one statement holds the
//! connection, a second async call on the same connection is outstanding, and
//! an unrelated `spawn_blocking` must still be served. Before the admission
//! gate the second call occupied the remaining blocking thread to wait for the
//! connection's mutex, and the unrelated task was starved.

use std::sync::Arc;
use std::time::Duration;

use toolu_orm_connection::DbConnection;

use crate::support::{self, HoldingRow};

/// The hold is five times the canary's budget, so the canary can only be served
/// by a blocking thread the gate kept free -- never by the hold finishing early.
const HOLD: Duration = Duration::from_millis(500);
const CANARY_BUDGET: Duration = Duration::from_millis(100);

#[test]
fn unrelated_blocking_work_is_served_while_a_statement_holds_the_connection()
-> Result<(), Box<dyn std::error::Error>> {
  let runtime = support::runtime(2)?;
  runtime.block_on(async {
    support::hold_for(HOLD.as_millis().try_into()?);
    let shared = Arc::new(support::seeded_connection()?);
    support::insert_label(&shared, "held")?;

    let holding = Arc::clone(&shared);
    let holder = tokio::spawn(async move {
      DbConnection::query_map::<HoldingRow>(&*holding, support::SELECT_LABEL, vec![]).await
    });
    // Signalled from inside the decode, on the blocking thread, with the
    // connection locked: waiting for it is proof the statement is in flight.
    support::DECODE_STARTED.notified().await;

    let waiting = Arc::clone(&shared);
    let queued =
      tokio::spawn(async move { DbConnection::execute_batch(&*waiting, "SELECT 1").await });
    // Long enough for the second call to reach the gate -- or, before the fix,
    // to take the second blocking thread and block on the connection's mutex.
    tokio::time::sleep(Duration::from_millis(25)).await;

    let unrelated = tokio::task::spawn_blocking(|| 7_i32);
    let answer = tokio::time::timeout(CANARY_BUDGET, unrelated)
      .await
      .map_err(|elapsed| {
        format!("unrelated blocking work was starved by the contended connection: {elapsed}")
      })??;
    assert_eq!(
      answer, 7,
      "the unrelated blocking task returns its own value"
    );

    let held = holder.await??;
    assert_eq!(
      held
        .iter()
        .map(|row| row.label.as_str())
        .collect::<Vec<_>>(),
      vec!["held"],
      "the holding statement still returns its row"
    );
    queued.await??;
    Ok(())
  })
}
