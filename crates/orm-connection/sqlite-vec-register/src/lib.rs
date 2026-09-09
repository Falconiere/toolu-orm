//! Register the statically linked `sqlite-vec` extension with rusqlite.
//!
//! Call [`register`] before opening any `rusqlite::Connection` that should see
//! the `vec0` module. Safe to call more than once.

use std::sync::Once;

use rusqlite::auto_extension::{RawAutoExtension, register_auto_extension};
use sqlite_vec::sqlite3_vec_init;

static REGISTER: Once = Once::new();

/// Register `sqlite3_vec_init` as a process-wide SQLite auto-extension.
///
/// Connections opened after this call have `vec0` available without a
/// per-connection `load_extension`. Idempotent.
///
/// # Panics
///
/// Panics if SQLite rejects the registration (non-zero return from
/// `sqlite3_auto_extension`). A failed registration leaves every later
/// `vec0` statement as a confusing missing-module error, so failing loud
/// here is preferable.
pub fn register() {
  REGISTER.call_once(|| {
    // SAFETY: The `sqlite-vec` crate declares `sqlite3_vec_init` as
    // `unsafe extern "C" fn()` but the C symbol is a SQLite auto-extension
    // entrypoint (`RawAutoExtension`). The cast matches the upstream
    // sqlite-vec + rusqlite integration; `register_auto_extension` checks
    // the SQLite return code.
    let result = unsafe {
      let init = std::mem::transmute::<unsafe extern "C" fn(), RawAutoExtension>(sqlite3_vec_init);
      register_auto_extension(init)
    };
    if let Err(error) = result {
      panic!("sqlite-vec auto-extension registration failed: {error}");
    }
  });
}
