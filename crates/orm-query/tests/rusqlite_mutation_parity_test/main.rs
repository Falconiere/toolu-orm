//! `UPDATE`/`DELETE` `RETURNING`, `ON CONFLICT` index predicates and update
//! guards, against in-memory rusqlite. Also probes whether this bundled
//! SQLite accepts `ORDER BY`/`LIMIT` on those statements.

mod conflict;
mod limit;
mod returning;
mod support;
