//! What a node's value payload may be: its own value, or a shared handle's.
//!
//! One enum per arity keeps every existing constructor working — a plain
//! `Value` converts in — while giving the renderer the identity it needs to
//! reuse an index instead of allocating a second one.

use crate::value::Value;

use super::handle::{BindingId, SharedBind, SharedBindList};

/// One value, owned by the node or shared through a handle.
pub(crate) enum BindSource {
  /// The node's own value; every occurrence binds separately.
  Owned(Value),
  /// A handle's value; occurrences of one handle share a placeholder.
  Shared(SharedBind),
}

impl BindSource {
  /// The identity to reuse an index under, or `None` when the node owns its
  /// value and each occurrence binds on its own.
  pub(crate) fn id(&self) -> Option<BindingId> {
    match self {
      Self::Owned(_) => None,
      Self::Shared(handle) => Some(handle.id()),
    }
  }

  /// The value this source binds.
  pub(crate) fn value(&self) -> &Value {
    match self {
      Self::Owned(value) => value,
      Self::Shared(handle) => handle.value(),
    }
  }
}

impl From<Value> for BindSource {
  fn from(value: Value) -> Self {
    Self::Owned(value)
  }
}

impl From<&SharedBind> for BindSource {
  fn from(handle: &SharedBind) -> Self {
    Self::Shared(handle.clone())
  }
}

/// A list of values, owned by the node or shared through a handle.
pub(crate) enum ListSource {
  /// The node's own values; every occurrence binds them again.
  Owned(Vec<Value>),
  /// A handle's values; occurrences of one handle share one placeholder run.
  Shared(SharedBindList),
}

impl ListSource {
  /// The identity to reuse a placeholder run under; see [`BindSource::id`].
  pub(crate) fn id(&self) -> Option<BindingId> {
    match self {
      Self::Owned(_) => None,
      Self::Shared(handle) => Some(handle.id()),
    }
  }

  /// The values this source binds, in placeholder order.
  pub(crate) fn values(&self) -> &[Value] {
    match self {
      Self::Owned(values) => values,
      Self::Shared(handle) => handle.values(),
    }
  }
}

impl From<Vec<Value>> for ListSource {
  fn from(values: Vec<Value>) -> Self {
    Self::Owned(values)
  }
}

impl From<&SharedBindList> for ListSource {
  fn from(handle: &SharedBindList) -> Self {
    Self::Shared(handle.clone())
  }
}
