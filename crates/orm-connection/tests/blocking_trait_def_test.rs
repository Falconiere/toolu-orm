//! Verify that [`DbConnectionBlocking`] is public and usable from downstream
//! crates, with no driver feature and no async runtime in the picture.
//!
//! Like `DbConnection`, it is not `dyn`-compatible because `query_map` is
//! generic; callers use `&(impl DbConnectionBlocking)` or the concrete
//! connection type instead.

use toolu_orm_connection::{DbConnectionBlocking, DbError};
use toolu_orm_core::value::Value;

/// The statement methods are called through the bound, so this fails to compile
/// if a method, `Value`, or `DbError` is not reachable from a crate that depends
/// on `toolu-orm-connection` alone. It needs no database: reaching the call is
/// the assertion, and executing the statements is the rusqlite-only lane's job.
///
/// `query_map` is left to that lane because calling it needs a `FromRow` impl,
/// and `FromRow`'s shape depends on which driver features Cargo unified.
#[test]
fn trait_is_importable() {
  fn _writes(conn: &impl DbConnectionBlocking) -> Result<u64, DbError> {
    conn.execute_batch("CREATE TABLE t (id INTEGER PRIMARY KEY)")?;
    conn.execute_sql("INSERT INTO t (id) VALUES (?1)", vec![Value::Integer(1)])
  }
}
