//! The reusable-binding scenarios executed against a live Postgres, where a
//! reused binding is a repeated `$N`.

#[path = "../fixtures/pg_reusable_bind_db.rs"]
pub mod db;
#[path = "../fixtures/reusable_bind_seed.rs"]
pub mod seed;

mod clauses;
mod co_change;
