//! The opt-in synchronization specification for an external-content FTS5 table.
//!
//! SQLite leaves such an index's consistency to the application: `rebuild`
//! indexes the rows that exist when it runs and nothing after that. Declaring
//! [`Fts5Sync`] hands the three triggers that keep it correct to the
//! generator, which also means they survive every recreation.
//!
//! Only the safe 1:1 case: the triggers read the table `content = '…'` names,
//! address rows by `content_rowid = '…'`, and give each FTS column the value
//! of the same-named content column.

use serde::{Deserialize, Serialize};

/// Prefix for every generated trigger name. It keeps a generated trigger from
/// colliding with an application's own, and makes the three recognizable in
/// `sqlite_master`.
const TRIGGER_PREFIX: &str = "toolu_fts5_";

/// What an FTS5 table synchronizes, and how.
///
/// Recorded on [`crate::table::TableDef`] and in the snapshot, so the diff can
/// create the triggers for a new declaration, drop them when it is removed,
/// and replace them when the source, the rowid, or the indexed columns change.
/// Built by [`Fts5Table::sync_content`](super::Fts5Table::sync_content).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fts5Sync {
  /// The ordinary table the three triggers are attached to — what
  /// `content = '…'` names.
  pub content_table: String,
  /// The content-table column FTS5 addresses rows by — what
  /// `content_rowid = '…'` names.
  pub content_rowid: String,
  /// Every FTS5 column, in declaration order. Each trigger writes all of them,
  /// and each reads the same-named content column.
  pub columns: Vec<String>,
  /// The subset of [`Self::columns`] FTS5 actually indexes. Only these and
  /// [`Self::content_rowid`] arm the update trigger, so a write that touches
  /// nothing the index covers costs nothing.
  pub indexed_columns: Vec<String>,
}

/// The insert, delete and update trigger names for `fts_table`, in creation
/// order.
///
/// They key on the FTS table alone: it has exactly one declaration, so two FTS
/// tables sharing one content table still get distinct names, and a drop needs
/// nothing but the FTS table's name.
///
/// ```
/// use toolu_orm_core::fts5::sync_trigger_names;
///
/// assert_eq!(sync_trigger_names("memory_fts")[0], "toolu_fts5_memory_fts_insert");
/// ```
#[must_use]
pub fn sync_trigger_names(fts_table: &str) -> [String; 3] {
  [
    format!("{TRIGGER_PREFIX}{fts_table}_insert"),
    format!("{TRIGGER_PREFIX}{fts_table}_delete"),
    format!("{TRIGGER_PREFIX}{fts_table}_update"),
  ]
}
