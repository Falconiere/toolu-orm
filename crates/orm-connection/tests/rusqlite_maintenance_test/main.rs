//! The SQLite maintenance and inspection surface on a borrowed connection
//! (issue #115): `VACUUM INTO`, `PRAGMA quick_check`, `ATTACH`/`DETACH` with a
//! guaranteed cleanup, and typed `page_count` / `page_size` reads.
//!
//! `VACUUM INTO` and `ATTACH DATABASE` operate on filenames, so every test here
//! runs against a real database in a real temporary directory. Nothing is
//! mocked and nothing substitutes an in-memory database for the file under
//! test.
//!
//! Gated on `rusqlite` alone, not on the single-driver shape: none of this
//! surface touches `FromRow`, so it compiles under any driver combination that
//! includes rusqlite. Of the four CI lanes only the rusqlite-only one turns that
//! feature on for this crate, which is where these tests run.
#![cfg(feature = "rusqlite")]

#[path = "../fixtures/temp_db_dir.rs"]
pub mod temp_db_dir;

mod attach;
mod inspect;
mod raw_access;
mod snapshot;
