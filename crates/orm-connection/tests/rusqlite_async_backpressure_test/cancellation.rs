//! Dropping the awaiting future, on both sides of admission.
//!
//! Before admission nothing has been scheduled, so no SQL may run. After
//! admission the statement is already on a blocking thread and cannot be
//! aborted, so the permit -- owned by that closure -- must stay held until it
//! ends; otherwise a cancelling caller could admit a replacement that would sit
//! on the connection's mutex and burn a second blocking thread.

use std::sync::Arc;
use std::time::Duration;

use toolu_orm_connection::DbConnection;
use toolu_orm_core::value::Value;

use crate::rusqlite_rows::LabelRow;
use crate::support::{self, HoldingRow};

const HOLD: Duration = Duration::from_millis(500);
const CANCEL_AFTER: Duration = Duration::from_millis(50);
const CANARY_BUDGET: Duration = Duration::from_millis(100);

fn labels(rows: &[LabelRow]) -> Vec<&str> {
  rows.iter().map(|row| row.label.as_str()).collect()
}

/// A caller that gives up while queued never reaches `spawn_blocking`, so its
/// statement never runs -- and the permit it never held is not lost either.
#[test]
fn cancelling_before_admission_starts_no_sql() -> Result<(), Box<dyn std::error::Error>> {
  let runtime = support::runtime(2)?;
  runtime.block_on(async {
    support::hold_for(HOLD.as_millis().try_into()?);
    let shared = Arc::new(support::seeded_connection()?);
    support::insert_label(&shared, "held")?;

    let holding = Arc::clone(&shared);
    let holder = tokio::spawn(async move {
      DbConnection::query_map::<HoldingRow>(&*holding, support::SELECT_LABEL, vec![]).await
    });
    support::DECODE_STARTED.notified().await;

    let queued = tokio::time::timeout(
      CANCEL_AFTER,
      DbConnection::execute_sql(
        &*shared,
        support::INSERT_LABEL,
        vec![Value::Text("cancelled".to_owned())],
      ),
    )
    .await;
    assert!(
      queued.is_err(),
      "the insert must still be queued on the gate when its future is dropped"
    );

    holder.await??;
    let after: Vec<LabelRow> =
      DbConnection::query_map(&*shared, support::SELECT_LABEL, vec![]).await?;
    assert_eq!(
      labels(&after),
      vec!["held"],
      "the cancelled insert must not have run"
    );
    Ok(())
  })
}

/// A caller that gives up after admission leaves its statement running. The
/// permit rides in the blocking closure, so nothing else is admitted until that
/// statement ends and the second blocking thread stays available.
#[test]
fn cancelling_after_admission_keeps_the_gate_closed() -> Result<(), Box<dyn std::error::Error>> {
  let runtime = support::runtime(2)?;
  runtime.block_on(async {
    support::hold_for(HOLD.as_millis().try_into()?);
    let shared = Arc::new(support::seeded_connection()?);
    support::insert_label(&shared, "held")?;

    // One poll is enough to take the permit and start the blocking task.
    let mut holder = Box::pin(DbConnection::query_map::<HoldingRow>(
      &*shared,
      support::SELECT_LABEL,
      vec![],
    ));
    let polled = tokio::time::timeout(CANCEL_AFTER, &mut holder).await;
    assert!(
      polled.is_err(),
      "the holding statement must still be running when its future is dropped"
    );
    support::DECODE_STARTED.notified().await;
    drop(holder);

    let replacing = Arc::clone(&shared);
    let replacement = tokio::spawn(async move {
      DbConnection::query_map::<LabelRow>(&*replacing, support::SELECT_LABEL, vec![]).await
    });
    tokio::time::sleep(Duration::from_millis(25)).await;

    let unrelated = tokio::task::spawn_blocking(|| 7_i32);
    let answer = tokio::time::timeout(CANARY_BUDGET, unrelated)
      .await
      .map_err(|elapsed| {
        format!("the cancelled caller let a replacement occupy the blocking pool: {elapsed}")
      })??;
    assert_eq!(answer, 7, "unrelated blocking work is still served");

    let rows = replacement.await??;
    assert_eq!(
      labels(&rows),
      vec!["held"],
      "the replacement runs once the cancelled statement releases the connection"
    );
    Ok(())
  })
}
