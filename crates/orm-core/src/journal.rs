use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;

use crate::error::DbCoreError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Journal {
  pub version: u32,
  pub entries: Vec<JournalEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
  pub idx: u32,
  pub name: String,
  pub hash: String,
  pub created_at: u64,
}

impl Journal {
  #[must_use]
  pub fn empty() -> Self {
    Self {
      version: 1,
      entries: Vec::new(),
    }
  }

  pub fn add_entry(&mut self, name: &str, hash: &str) {
    let idx = u32::try_from(self.entries.len()).unwrap_or(u32::MAX);
    let created_at = std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .unwrap_or_default()
      .as_secs();
    self.entries.push(JournalEntry {
      idx,
      name: name.to_owned(),
      hash: hash.to_owned(),
      created_at,
    });
  }

  #[must_use]
  pub fn next_migration_number(&self) -> u32 {
    self
      .entries
      .iter()
      .filter_map(|e| e.name.split('_').next()?.parse::<u32>().ok())
      .max()
      .map_or(1, |n| n + 1)
  }

  #[must_use]
  pub fn latest_snapshot_name_owned(&self) -> Option<String> {
    self.entries.last().map(|e| {
      let base = e.name.strip_suffix(".sql").unwrap_or(&e.name);
      format!("{base}.snapshot.json")
    })
  }

  /// Reads a journal from a JSON file, returning an empty journal if the file does not exist.
  ///
  /// # Errors
  ///
  /// Returns [`DbCoreError::JournalRead`] if the file exists but cannot be read or parsed.
  pub fn read_from_path(path: &str) -> Result<Self, DbCoreError> {
    let p = Path::new(path);
    if !p.exists() {
      return Ok(Self::empty());
    }
    let content =
      std::fs::read_to_string(p).map_err(|e| DbCoreError::JournalRead(format!("{path}: {e}")))?;
    serde_json::from_str(&content).map_err(|e| DbCoreError::JournalRead(format!("{path}: {e}")))
  }

  /// Writes the journal to a JSON file.
  ///
  /// # Errors
  ///
  /// Returns [`DbCoreError::JournalWrite`] if serialization or file writing fails.
  pub fn write_to_path(&self, path: &str) -> Result<(), DbCoreError> {
    let json = serde_json::to_string_pretty(self)
      .map_err(|e| DbCoreError::JournalWrite(format!("{path}: {e}")))?;
    std::fs::write(path, json).map_err(|e| DbCoreError::JournalWrite(format!("{path}: {e}")))
  }
}

/// Computes a SHA-256 hash of the given content, prefixed with `sha256:`.
#[must_use]
pub fn compute_hash(content: &str) -> String {
  let mut hasher = Sha256::new();
  hasher.update(content.as_bytes());
  let result = hasher.finalize();
  format!("sha256:{result:x}")
}
