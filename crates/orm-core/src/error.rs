use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbCoreError {
  #[error("failed to read snapshot: {0}")]
  SnapshotRead(String),

  #[error("failed to write snapshot: {0}")]
  SnapshotWrite(String),

  #[error("failed to write migration: {0}")]
  MigrationWrite(String),

  #[error("row mapping error: {0}")]
  RowMapping(String),

  #[error("connection initialization error: {0}")]
  Connection(String),

  #[error(
    "cannot migrate virtual table \"{table}\" in place: {reason}; SQLite has no ALTER TABLE for \
     virtual tables — drop, recreate and repopulate it in a hand-written migration"
  )]
  VirtualTableChange { table: String, reason: String },

  #[error(
    "{function} is a SQLite FTS5 feature with no {dialect} equivalent; build this query for \
     SQLite, or write the Postgres full-text form (@@ / to_tsquery / ts_rank) yourself"
  )]
  Fts5UnsupportedDialect {
    function: String,
    dialect: &'static str,
  },

  #[error("invalid FTS5 argument for {function}: {reason}")]
  Fts5InvalidArgument { function: String, reason: String },

  #[error("failed to read journal: {0}")]
  JournalRead(String),

  #[error("failed to write journal: {0}")]
  JournalWrite(String),
}
