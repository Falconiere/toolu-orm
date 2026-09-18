//! [`BoundParams`]: the statement-wide parameter buffer and its binding ledger.

use std::collections::HashMap;
use std::ops::Range;

use crate::value::Value;

use super::handle::BindingId;
use super::source::{BindSource, ListSource};

/// The values a statement has bound so far, plus the index each shared handle
/// took.
///
/// Every renderer threads one of these and asks it for the next placeholder,
/// which is always [`next_index`](Self::next_index) — `len() + 1`. That is the
/// discipline that makes numbering across a clause or statement boundary free:
/// position lives in the buffer, so no node adds an offset of its own.
/// [`nested`](Self::nested) is the single place that re-bases it.
///
/// A shared handle is the one thing that does not push on every occurrence:
/// its first use allocates and records an index, and later uses in the same
/// buffer render that index again.
pub struct BoundParams {
  /// The bound values, in placeholder order.
  values: Vec<Value>,
  /// The absolute, 1-based index each handle seen so far took.
  shared: HashMap<BindingId, usize>,
}

impl BoundParams {
  /// An empty buffer with an empty ledger.
  #[must_use]
  pub fn new() -> Self {
    Self {
      values: Vec::new(),
      shared: HashMap::new(),
    }
  }

  /// How many values have been bound.
  #[must_use]
  pub fn len(&self) -> usize {
    self.values.len()
  }

  /// Whether nothing has been bound yet.
  #[must_use]
  pub fn is_empty(&self) -> bool {
    self.values.is_empty()
  }

  /// The index the next placeholder written into this buffer takes.
  #[must_use]
  pub fn next_index(&self) -> usize {
    self.values.len() + 1
  }

  /// Binds one more value and returns the index it took; no handle can reuse
  /// it.
  pub fn push(&mut self, value: Value) -> usize {
    let index = self.next_index();
    self.values.push(value);
    index
  }

  /// The bound values, in placeholder order.
  #[must_use]
  pub fn into_values(self) -> Vec<Value> {
    self.values
  }

  /// Renders a nested statement whose first placeholder is `start`, appending
  /// only what that statement binds and sharing this buffer's ledger.
  ///
  /// Every renderer numbers from [`next_index`](Self::next_index), which
  /// counts from the buffer's own start, so the frame stands in for the
  /// `start - 1` values already emitted, renders, and drops the stand-ins.
  /// They are never observed: the renderers read the length and never the
  /// contents.
  ///
  /// **`start` must be where the frame's values will actually land.** That is
  /// [`next_index`](Self::next_index) when appending to a live buffer — every
  /// splice in this crate passes exactly that — or, on an empty buffer, the
  /// absolute index the caller will splice the result at, which is how
  /// `Expr::to_sql_fragment_for(start, …)` renders a standalone fragment.
  /// Any other value numbers placeholders away from the values behind them.
  ///
  /// The ledger travels into the child and back, which is what lets a handle
  /// first used *inside* the nested statement be reused outside it — the index
  /// the child recorded is the absolute one, because the child's own values
  /// land at exactly those positions.
  pub fn nested<R>(&mut self, start: usize, render: impl FnOnce(&mut Self) -> R) -> R {
    let emitted = start.saturating_sub(1);
    let mut child = Self {
      values: vec![Value::Null; emitted],
      shared: std::mem::take(&mut self.shared),
    };

    let rendered = render(&mut child);

    self.shared = child.shared;
    self.values.extend(child.values.into_iter().skip(emitted));
    rendered
  }

  /// The placeholder index for `source`, binding its value if this is the
  /// first time the buffer has seen it.
  pub(crate) fn bind(&mut self, source: &BindSource) -> usize {
    let binding = source.id();
    if let Some(index) = binding.and_then(|id| self.shared.get(&id).copied()) {
      return index;
    }

    let index = self.push(source.value().clone());
    if let Some(id) = binding {
      self.shared.insert(id, index);
    }
    index
  }

  /// The placeholder indices for `source`, binding its values if this is the
  /// first time the buffer has seen them.
  ///
  /// The run is contiguous, so the ledger stores only its first index. An
  /// empty source binds nothing and yields an empty range: the caller renders
  /// the constant an empty `IN` list has always rendered.
  pub(crate) fn bind_list(&mut self, source: &ListSource) -> Range<usize> {
    let values = source.values();
    if values.is_empty() {
      return 0..0;
    }

    let binding = source.id();
    if let Some(first) = binding.and_then(|id| self.shared.get(&id).copied()) {
      return first..first + values.len();
    }

    let first = self.next_index();
    self.values.extend_from_slice(values);
    if let Some(id) = binding {
      self.shared.insert(id, first);
    }
    first..first + values.len()
  }
}

impl Default for BoundParams {
  fn default() -> Self {
    Self::new()
  }
}

/// So a clause that already holds a rendered fragment's values can append them
/// exactly as it appended them to a plain `Vec<Value>`.
impl Extend<Value> for BoundParams {
  fn extend<I: IntoIterator<Item = Value>>(&mut self, values: I) {
    self.values.extend(values);
  }
}
