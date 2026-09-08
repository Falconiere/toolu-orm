//! Bitflag constraints parsed from `#[column(...)]`.
//!
//! One byte instead of one bool per constraint, so the parsed column struct
//! stays under Clippy's `struct_excessive_bools` limit.

#[derive(Clone, Copy, Default)]
pub struct ColumnFlags(u8);

const FLAG_PRIMARY_KEY: u8 = 1 << 0;
const FLAG_NOT_NULL: u8 = 1 << 1;
const FLAG_UNIQUE: u8 = 1 << 2;
const FLAG_AS_TEXT: u8 = 1 << 3;
const FLAG_UNINDEXED: u8 = 1 << 4;
const FLAG_PARTITION_KEY: u8 = 1 << 5;
const FLAG_AUXILIARY: u8 = 1 << 6;

impl ColumnFlags {
  pub const fn primary_key(self) -> bool {
    self.has(FLAG_PRIMARY_KEY)
  }
  pub const fn not_null(self) -> bool {
    self.has(FLAG_NOT_NULL)
  }
  pub const fn unique(self) -> bool {
    self.has(FLAG_UNIQUE)
  }
  pub const fn as_text(self) -> bool {
    self.has(FLAG_AS_TEXT)
  }
  /// FTS5 `UNINDEXED`: stored but not searchable. Only `#[fts5_table]` reads it.
  pub const fn unindexed(self) -> bool {
    self.has(FLAG_UNINDEXED)
  }
  /// vec0 `partition key`: the index is sharded on it. Only `#[vec0_table]`.
  pub const fn partition_key(self) -> bool {
    self.has(FLAG_PARTITION_KEY)
  }
  /// vec0 `+column`: stored beside the index, never filtered on. Only `#[vec0_table]`.
  pub const fn auxiliary(self) -> bool {
    self.has(FLAG_AUXILIARY)
  }

  pub(super) fn set_primary_key(&mut self) {
    self.0 |= FLAG_PRIMARY_KEY;
  }
  pub(super) fn set_not_null(&mut self) {
    self.0 |= FLAG_NOT_NULL;
  }
  pub(super) fn set_unique(&mut self) {
    self.0 |= FLAG_UNIQUE;
  }
  pub(super) fn set_as_text(&mut self) {
    self.0 |= FLAG_AS_TEXT;
  }
  pub(super) fn set_unindexed(&mut self) {
    self.0 |= FLAG_UNINDEXED;
  }
  pub(super) fn set_partition_key(&mut self) {
    self.0 |= FLAG_PARTITION_KEY;
  }
  pub(super) fn set_auxiliary(&mut self) {
    self.0 |= FLAG_AUXILIARY;
  }

  const fn has(self, flag: u8) -> bool {
    (self.0 & flag) != 0
  }
}
