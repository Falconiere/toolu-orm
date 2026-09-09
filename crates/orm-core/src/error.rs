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
     virtual tables. When the FTS5 definition sets content = '…' to an ordinary table in the \
     schema whose columns cover the index, generate emits drop + recreate + INSERT INTO … \
     VALUES('rebuild'). Otherwise drop, recreate, and repopulate in a hand-written migration"
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

  /// `vec0` parses its own constructor arguments with a scanner that has no
  /// quoting, so a name outside `[A-Za-z][A-Za-z0-9_]*` cannot be rendered
  /// safely at all — unlike FTS5, where quoting the name is enough.
  #[error(
    "\"{ident}\" cannot be a vec0 {context}: vec0 parses its own arguments and has no quoting, \
     so the name must match [A-Za-z][A-Za-z0-9_]*"
  )]
  InvalidVec0Identifier {
    context: &'static str,
    ident: String,
  },

  #[error("vec0 column \"{column}\" is a bit vector, and a bit vector has no distance_metric")]
  Vec0BitDistanceMetric { column: String },

  #[error("expected a {expected}-element embedding, got {actual}")]
  VectorDimension { expected: u32, actual: usize },

  #[error(
    "{feature} is a SQLite sqlite-vec feature with no {dialect} equivalent; build this query for \
     SQLite, or write the pgvector form (<-> / <=> / <#>) yourself"
  )]
  Vec0UnsupportedDialect {
    feature: String,
    dialect: &'static str,
  },

  #[error("invalid vec0 argument for {feature}: {reason}")]
  Vec0InvalidArgument { feature: String, reason: String },

  #[error(
    "{function} is a Postgres full-text feature with no {dialect} equivalent; build this query for \
     Postgres, or use the SQLite FTS5 surface (MATCH / bm25) instead"
  )]
  PgFtsUnsupportedDialect {
    function: String,
    dialect: &'static str,
  },

  #[error("invalid Postgres FTS argument for {function}: {reason}")]
  PgFtsInvalidArgument { function: String, reason: String },

  #[error(
    "{feature} is a Postgres pgvector feature with no {dialect} equivalent; build this query for \
     Postgres, or use the sqlite-vec KNN surface (MATCH / k / distance) instead"
  )]
  PgVectorUnsupportedDialect {
    feature: String,
    dialect: &'static str,
  },

  #[error("invalid pgvector argument for {feature}: {reason}")]
  PgVectorInvalidArgument { feature: String, reason: String },

  #[error("failed to read journal: {0}")]
  JournalRead(String),

  #[error("failed to write journal: {0}")]
  JournalWrite(String),
}
