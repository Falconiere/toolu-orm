//! Verify that [`DbConnectionBlocking`] is public and usable from downstream
//! crates, with no driver feature and no async runtime in the picture.
//!
//! Like `DbConnection`, it is not `dyn`-compatible because `query_map` is
//! generic; callers use `&(impl DbConnectionBlocking)` or the concrete
//! connection type instead.

use toolu_orm_connection::DbConnectionBlocking;

#[test]
fn trait_is_importable() {
  fn _accepts_conn(_conn: &impl DbConnectionBlocking) {}
}
