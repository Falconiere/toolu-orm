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

  #[error("failed to read journal: {0}")]
  JournalRead(String),

  #[error("failed to write journal: {0}")]
  JournalWrite(String),
}
