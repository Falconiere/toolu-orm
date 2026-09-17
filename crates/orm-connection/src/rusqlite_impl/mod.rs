//! rusqlite backend: sync SQLite, native for `DbConnectionBlocking` and wrapped
//! with `tokio::task::spawn_blocking` for the async `DbConnection`.
//!
//! For consumers that run with embedded SQLite and no network database
//! (for example remote workers).
//!
//! A `rusqlite::Connection` runs one statement at a time, so the async surface
//! admits one operation at a time through a shared semaphore before it calls
//! `spawn_blocking`: a queued caller waits on its own task instead of on a
//! thread from tokio's finite blocking pool. Direct `DbConnectionBlocking`
//! callers are not gated -- a synchronous caller cannot await a permit, and it
//! runs on the thread it is already on -- so they take the mutex directly.

mod asynchronous;
mod blocking;
mod connection;
mod params;

pub use connection::RusqliteConnection;
