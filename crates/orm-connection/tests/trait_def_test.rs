//! Verify that [`DbConnection`] is public and usable from downstream crates.
//!
//! `DbConnection` is not `dyn`-compatible because `query_map` is generic; callers
//! use `&(impl DbConnection)` or concrete connection types instead.

use toolu_orm_connection::DbConnection;

#[test]
fn trait_is_importable() {
  fn _accepts_conn(_conn: &impl DbConnection) {}
}
