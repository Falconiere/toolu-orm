//! Opening the pair of databases and attaching one to the other, shared by this
//! binary's scenario modules.

use std::path::PathBuf;

use toolu_orm_connection::{AttachedDatabase, SqliteMaintenance};

use super::db::{open_target, seed_source};
use super::temp_db_dir::TempDbDir;

/// The schema the older store is attached under, on the target's connection.
pub const OLD: &str = "old";

/// A target connection plus the older store's file, both alive for as long as
/// this value is.
///
/// The directory is deleted on drop, so it must outlive every copy. So must the
/// [`AttachedDatabase`] guard [`attach`](Self::attach) returns, which detaches
/// on drop — and SQLite refuses to detach a database an open transaction has
/// written, which is why no test here wraps its copy in one.
pub struct Attached {
  pub conn: rusqlite::Connection,
  dir: TempDbDir,
}

impl Attached {
  /// Seed a fresh pair of files under `label` and open the target.
  ///
  /// # Errors
  ///
  /// The I/O or rusqlite error that prevented the setup.
  pub fn open(label: &str) -> Result<Self, Box<dyn std::error::Error>> {
    let dir = TempDbDir::new(label)?;
    seed_source(&dir.file("old.db"))?;
    let conn = open_target(&dir.file("main.db"))?;
    Ok(Self { conn, dir })
  }

  /// `ATTACH` the older store under [`OLD`], borrowing the connection.
  ///
  /// # Errors
  ///
  /// The maintenance error `ATTACH DATABASE` reported.
  pub fn attach(&self) -> Result<AttachedDatabase<'_>, Box<dyn std::error::Error>> {
    let path: PathBuf = self.dir.file("old.db");
    Ok(self.conn.attach_database(&path, OLD)?)
  }
}
