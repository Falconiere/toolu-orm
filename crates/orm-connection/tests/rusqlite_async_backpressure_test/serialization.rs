//! The gate bounds scheduling; it must not change what the connection does.
//!
//! Concurrent async callers still serialize on one connection and every write
//! lands, and a direct `DbConnectionBlocking` caller -- which never takes a
//! permit -- keeps working while async callers are queued behind the gate.

use std::sync::Arc;
use std::time::Duration;

use toolu_orm_connection::DbConnection;
use toolu_orm_core::value::Value;

use crate::blocking_conn::scalar;
use crate::support::{self, HoldingRow};

const WRITERS: i64 = 4;
const ROWS_PER_WRITER: i64 = 25;
const SHORT_HOLD: Duration = Duration::from_millis(200);

/// Every insert from every task lands exactly once: one permit at a time serializes
/// the async callers, and the connection's mutex serializes the statements.
#[test]
fn concurrent_async_writers_all_land() -> Result<(), Box<dyn std::error::Error>> {
  let runtime = support::runtime(4)?;
  runtime.block_on(async {
    let shared = Arc::new(support::seeded_connection()?);

    let mut writers = Vec::new();
    for writer in 0..WRITERS {
      let target = Arc::clone(&shared);
      writers.push(tokio::spawn(async move {
        for row in 0..ROWS_PER_WRITER {
          DbConnection::execute_sql(
            &*target,
            support::INSERT_LABEL,
            vec![Value::Text(format!("writer-{writer}-row-{row}"))],
          )
          .await?;
        }
        Ok::<(), toolu_orm_connection::DbError>(())
      }));
    }
    for writer in writers {
      writer.await??;
    }

    let total = WRITERS * ROWS_PER_WRITER;
    assert_eq!(scalar(&shared, "SELECT COUNT(*) FROM gated")?, total);
    assert_eq!(
      scalar(&shared, "SELECT COUNT(DISTINCT label) FROM gated")?,
      total,
      "no write is lost or duplicated"
    );
    Ok(())
  })
}

/// The blocking surface bypasses the gate: it waits for the connection's mutex
/// only, so a queue of async callers cannot shut it out.
#[test]
fn a_direct_blocking_caller_lands_while_async_callers_queue()
-> Result<(), Box<dyn std::error::Error>> {
  let runtime = support::runtime(2)?;
  runtime.block_on(async {
    support::hold_for(SHORT_HOLD.as_millis().try_into()?);
    let shared = Arc::new(support::seeded_connection()?);
    support::insert_label(&shared, "held")?;

    let holding = Arc::clone(&shared);
    let holder = tokio::spawn(async move {
      DbConnection::query_map::<HoldingRow>(&*holding, support::SELECT_LABEL, vec![]).await
    });
    support::DECODE_STARTED.notified().await;

    let mut queued = Vec::new();
    for index in 0..2 {
      let target = Arc::clone(&shared);
      queued.push(tokio::spawn(async move {
        DbConnection::execute_sql(
          &*target,
          support::INSERT_LABEL,
          vec![Value::Text(format!("async-{index}"))],
        )
        .await
      }));
    }

    // Its own OS thread, no runtime and no permit: it contends on the mutex.
    let direct = Arc::clone(&shared);
    let blocking_writer = std::thread::spawn(move || support::insert_label(&direct, "blocking"));

    holder.await??;
    for insert in queued {
      assert_eq!(insert.await??, 1);
    }
    let landed = blocking_writer
      .join()
      .map_err(|payload| format!("the blocking writer thread panicked: {payload:?}"))??;
    assert_eq!(landed, 1, "the direct blocking insert lands");

    assert_eq!(scalar(&shared, "SELECT COUNT(*) FROM gated")?, 4);
    assert_eq!(
      scalar(
        &shared,
        "SELECT COUNT(*) FROM gated WHERE label = 'blocking'"
      )?,
      1
    );
    Ok(())
  })
}
