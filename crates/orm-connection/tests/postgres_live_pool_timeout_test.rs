//! `PgConfig::checkout_timeout` against a live, one-slot Postgres pool:
//! timeout fires near the deadline, an in-time release still succeeds, a
//! timed-out waiter does not leak pool capacity, and `None` truly opts out
//! of the internal wait bound.
//!
//! Needs `docker compose -f docker-compose.test.yaml up -d --wait` and
//! `TEST_DB_PORT=5434` locally (CI provides a service container). Compiles
//! only on the five-crate postgres lane, where orm-core has the
//! postgres+libsql `FromRow` shape — matched here even though these tests
//! never decode a row, so the binary is gated the same way as its
//! `postgres_live_*_test.rs` siblings. No shared `pg_live` fixture is
//! pulled in: none of it (schema setup, `CountRow`) is needed for
//! checkout-behavior assertions, only a fresh one-slot pool per test.
#![cfg(all(feature = "libsql", feature = "postgres"))]

use std::time::{Duration, Instant};

use toolu_orm_connection::{DbError, PgConfig, PgDatabase};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A fresh, single-slot pool dedicated to one test against the shared
/// `toolu` test database — no schema or table state is needed, since every
/// assertion here is about checkout behavior, not query results.
async fn one_slot_pool(
  checkout_timeout: Option<Duration>,
) -> Result<PgDatabase, Box<dyn std::error::Error>> {
  let db = PgDatabase::init(&PgConfig {
    max_connections: 1,
    checkout_timeout,
    ..PgConfig::for_test("toolu")
  })
  .await?;
  Ok(db)
}

#[tokio::test]
async fn checkout_times_out_near_the_configured_bound() -> TestResult {
  let db = one_slot_pool(Some(Duration::from_millis(200))).await?;
  let held = db.connect().await?;

  let start = Instant::now();
  let result = db.connect().await;
  let elapsed = start.elapsed();

  let Err(DbError::Pool(msg)) = result else {
    return Err("expected a held slot to return DbError::Pool on timeout".into());
  };
  assert!(
    msg.contains("timed out"),
    "expected a timeout-indicating message, got: {msg}"
  );
  assert!(
    elapsed >= Duration::from_millis(200),
    "checkout returned before the configured timeout: {elapsed:?}"
  );
  assert!(
    elapsed < Duration::from_secs(2),
    "checkout took far longer than the configured timeout: {elapsed:?}"
  );

  drop(held);
  Ok(())
}

#[tokio::test]
async fn release_before_deadline_lets_a_waiter_succeed() -> TestResult {
  let db = one_slot_pool(Some(Duration::from_secs(1))).await?;
  let held = db.connect().await?;

  let waiter_db = db.clone();
  let waiter = tokio::spawn(async move { waiter_db.connect().await });

  // Release well before the 1s deadline so the waiter's wait is satisfied,
  // not timed out.
  tokio::time::sleep(Duration::from_millis(100)).await;
  drop(held);

  let result = waiter
    .await
    .map_err(|e| format!("waiter task panicked: {e}"))?;
  match result {
    Ok(_conn) => Ok(()),
    Err(e) => Err(format!("expected the waiter to succeed once the slot freed, got: {e}").into()),
  }
}

#[tokio::test]
async fn timed_out_waiter_does_not_leak_pool_capacity() -> TestResult {
  let db = one_slot_pool(Some(Duration::from_millis(200))).await?;
  let held = db.connect().await?;

  let timed_out = db.connect().await;
  assert!(
    matches!(timed_out, Err(DbError::Pool(_))),
    "expected the held slot to produce a timeout error first"
  );

  drop(held);

  let after = db.connect().await;
  assert!(
    after.is_ok(),
    "a later checkout should succeed once capacity was released — the timed-out waiter must not have leaked it"
  );
  Ok(())
}

#[tokio::test]
async fn checkout_timeout_none_disables_the_wait_bound() -> TestResult {
  let db = one_slot_pool(None).await?;
  let held = db.connect().await?;

  // Wrap the call in an OUTER timeout from the test itself, so the test
  // cannot hang indefinitely even if the None opt-out somehow failed to
  // disable the internal bound. The internal call has no wait_timeout, so
  // it must still be pending when the outer wrapper's shorter deadline
  // fires.
  let outer = tokio::time::timeout(Duration::from_millis(300), db.connect()).await;
  assert!(
    outer.is_err(),
    "checkout_timeout: None should leave the wait unbounded internally, so the outer \
     wrapper — not an internal timeout — should be what expires here"
  );

  drop(held);

  let after = db.connect().await;
  assert!(
    after.is_ok(),
    "a checkout should succeed once the slot is released, proving the pool is still usable"
  );
  Ok(())
}
