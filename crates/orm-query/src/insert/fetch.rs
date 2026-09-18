//! `RETURNING` row fetches for [`super::InsertBuilder`].
//!
//! Only compiled when exactly one driver feature is active, like every other
//! executor-bound method in this crate.

use crate::exec_helpers::impl_returning_fetch;

impl_returning_fetch!(super::InsertBuilder, "INSERT");
