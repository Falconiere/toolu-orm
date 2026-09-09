//! Register the statically linked `sqlite-vec` extension with rusqlite.
//!
//! Call [`register`] before opening any `rusqlite::Connection` that should see
//! the `vec0` module. Safe to call more than once.

use std::sync::OnceLock;

use rusqlite::auto_extension::{RawAutoExtension, register_auto_extension};
use sqlite_vec::sqlite3_vec_init;

static REGISTERED: OnceLock<Result<(), String>> = OnceLock::new();

/// Register `sqlite3_vec_init` as a process-wide SQLite auto-extension.
///
/// Connections opened after a successful call have `vec0` available without a
/// per-connection `load_extension`. Idempotent: later calls return the same
/// outcome as the first.
///
/// # Errors
///
/// Returns an error when SQLite rejects the registration (non-zero return from
/// `sqlite3_auto_extension`). Callers should fail before running `vec0` DDL.
pub fn register() -> Result<(), String> {
  REGISTERED
    .get_or_init(|| {
      // SAFETY: The `sqlite-vec` crate declares `sqlite3_vec_init` as
      // `unsafe extern "C" fn()` but the C symbol is a SQLite auto-extension
      // entrypoint (`RawAutoExtension`). The cast matches the upstream
      // sqlite-vec + rusqlite integration; `register_auto_extension` checks
      // the SQLite return code.
      unsafe {
        let init =
          std::mem::transmute::<unsafe extern "C" fn(), RawAutoExtension>(sqlite3_vec_init);
        register_auto_extension(init).map_err(|error| error.to_string())
      }
    })
    .clone()
}
