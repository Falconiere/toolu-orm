//! Named portability requirements checked before structured operations execute.

use toolu_orm_core::dialect::Dialect;

use crate::DbError;

/// A backend guarantee an operation requires, not a claim about its SQL syntax.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
  /// Native key-based MERGE (not unique-key conflict handling).
  KeyMerge,
  /// Unique-key conflict handling, including ignore and replace shorthands.
  OnConflict,
  /// Rows returned by ordinary INSERT, UPDATE, or DELETE.
  DmlReturning,
  /// Enforced primary keys.
  PrimaryKey,
  /// Enforced unique constraints.
  UniqueConstraint,
  /// Indexes that enforce uniqueness.
  UniqueIndex,
  /// Enforced foreign keys.
  ForeignKey,
  /// Enforced NOT NULL declarations.
  NotNull,
  /// Enforced CHECK declarations.
  CheckConstraint,
  /// General multi-statement transactions, including schema operations.
  MultiStatementTransaction,
}

impl Capability {
  /// Stable capability identifier for diagnostics.
  #[must_use]
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::KeyMerge => "key_merge",
      Self::OnConflict => "on_conflict",
      Self::DmlReturning => "dml_returning",
      Self::PrimaryKey => "primary_key",
      Self::UniqueConstraint => "unique_constraint",
      Self::UniqueIndex => "unique_index",
      Self::ForeignKey => "foreign_key",
      Self::NotNull => "not_null",
      Self::CheckConstraint => "check_constraint",
      Self::MultiStatementTransaction => "multi_statement_transaction",
    }
  }

  /// Explicit alternative; none silently promises the missing guarantee.
  #[must_use]
  pub const fn alternative(self) -> &'static str {
    match self {
      Self::KeyMerge => {
        "use native MERGE on Lance or PostgreSQL 15+; SQLite ON CONFLICT is a separate API requiring unique constraints"
      },
      Self::OnConflict => {
        "use PostgreSQL or SQLite for unique-key conflict handling; explicit MergeBuilder has different semantics and does not enforce uniqueness"
      },
      Self::DmlReturning => {
        "execute without RETURNING, then SELECT explicitly if needed; the separate read is not atomic with the write"
      },
      Self::PrimaryKey | Self::UniqueConstraint | Self::UniqueIndex => {
        "use PostgreSQL or SQLite for enforced uniqueness; a Lance scalar index does not enforce it"
      },
      Self::ForeignKey => {
        "use PostgreSQL or SQLite with foreign-key enforcement enabled for referential integrity"
      },
      Self::NotNull | Self::CheckConstraint => {
        "use PostgreSQL or SQLite for database-enforced constraints; application validation alone is not equivalent"
      },
      Self::MultiStatementTransaction => {
        "use PostgreSQL or SQLite transactions for multi-statement atomicity; individual Lance operations do not provide that guarantee"
      },
    }
  }
}

/// Check structured requirements before any SQL or side effects.
///
/// Empty requirements pass (for example an ordinary filtered SELECT). SQLite
/// refuses native KeyMerge; PostgreSQL passes this coarse backend check. Configuration and
/// statement validity still matter. Raw SQL and fragments are never inspected.
/// Schema callers must invoke this before mutation; it is not schema validation.
///
/// # Errors
///
/// Returns the first unsupported requirement with a named alternative.
pub fn require_capabilities(backend: Dialect, requirements: &[Capability]) -> Result<(), DbError> {
  for &capability in requirements {
    if (capability == Capability::KeyMerge && backend == Dialect::Sqlite)
      || (capability != Capability::KeyMerge && backend == Dialect::Lance)
    {
      return Err(DbError::UnsupportedCapability {
        backend,
        capability,
      });
    }
  }
  Ok(())
}
