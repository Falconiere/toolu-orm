//! `Vec0Table` inside orm-core: what it renders, what it refuses, the DDL on
//! each dialect, and what survives a snapshot.

mod ddl;
mod refusals;
mod rendering;
mod snapshot_round_trip;
mod vec0_fixture;

pub(crate) use vec0_fixture::{create_sql, memory_vec, TestResult};
