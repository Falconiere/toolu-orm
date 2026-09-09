//! Register the statically linked `sqlite-vec` extension with rusqlite.
//!
//! Call [`register`] before opening any `rusqlite::Connection` that should see
//! the `vec0` module. Safe to call more than once.

use std::sync::Once;

use rusqlite::ffi::{sqlite3, sqlite3_api_routines, sqlite3_auto_extension};
use sqlite_vec::sqlite3_vec_init;

type SqliteEntryPoint = unsafe extern "C" fn(
  db: *mut sqlite3,
  pz_err_msg: *mut *mut std::os::raw::c_char,
  p_api: *const sqlite3_api_routines,
) -> std::os::raw::c_int;

static REGISTER: Once = Once::new();

/// Register `sqlite3_vec_init` as a process-wide SQLite auto-extension.
///
/// Connections opened after this call have `vec0` available without a
/// per-connection `load_extension`. Idempotent.
pub fn register() {
  REGISTER.call_once(|| {
    // SAFETY: `sqlite3_vec_init` is the official entrypoint from the
    // statically linked `sqlite-vec` crate; registering it once is the
    // documented integration with rusqlite's bundled libsqlite3.
    unsafe {
      let init = std::mem::transmute::<*const (), SqliteEntryPoint>(sqlite3_vec_init as *const ());
      sqlite3_auto_extension(Some(init));
    }
  });
}
