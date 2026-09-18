//! The reusable binding handles: [`SharedBind`] and [`SharedBindList`].

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::value::Value;

/// The identity of one reusable binding.
///
/// Drawn from a process-wide counter rather than from the payload's address:
/// an allocator may hand a dropped handle's address to a later one, which
/// would silently merge two bindings the caller built to be independent.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) struct BindingId(u64);

/// Hands out one identity per handle constructed.
static NEXT_BINDING_ID: AtomicU64 = AtomicU64::new(1);

impl BindingId {
  /// The next identity.
  ///
  /// `Relaxed` is sufficient: `fetch_add` on a single atomic is totally
  /// ordered with itself, and no other memory is published through it.
  fn next() -> Self {
    Self(NEXT_BINDING_ID.fetch_add(1, Ordering::Relaxed))
  }
}

/// A payload plus the identity that decides whether two uses are the same bind.
struct SharedCell<T> {
  id: BindingId,
  payload: T,
}

impl<T> SharedCell<T> {
  fn new(payload: T) -> Arc<Self> {
    Arc::new(Self {
      id: BindingId::next(),
      payload,
    })
  }
}

/// One value bound once, however many predicates reference it.
///
/// Every occurrence built from one handle renders the *same* placeholder, so a
/// value repeated across `AND` / `OR` costs one parameter, not one per use.
///
/// **Identity is the handle, never the value:** two handles over equal values
/// bind twice, a clone binds once. A handle carries its own value, so it is
/// never unbound and belongs to no statement — the first occurrence in a
/// render allocates it and the rest reuse that index.
///
/// ```
/// # use toolu_orm_core::{column::Text, dialect::Dialect, expr::SharedBind};
/// # use toolu_orm_core::query_column::{Column, SharedOps};
/// const SRC: Column<Text> = Column::new("edges", "src_id");
/// const DST: Column<Text> = Column::new("edges", "dst_id");
///
/// let node = SharedBind::new("file:a.rs");
/// let (sql, params) = SRC
///   .eq_shared(&node)
///   .or(DST.eq_shared(&node))
///   .to_sql_fragment_for(1, Dialect::Sqlite);
/// assert_eq!(sql, r#"("edges"."src_id" = ?1 OR "edges"."dst_id" = ?1)"#);
/// assert_eq!(params.len(), 1);
/// ```
pub struct SharedBind {
  cell: Arc<SharedCell<Value>>,
}

/// Cloning shares the binding: a clone and its original render one placeholder.
impl Clone for SharedBind {
  fn clone(&self) -> Self {
    Self {
      cell: Arc::clone(&self.cell),
    }
  }
}

impl fmt::Debug for SharedBind {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("SharedBind")
      .field("id", &self.cell.id)
      .field("value", &self.cell.payload)
      .finish()
  }
}

impl SharedBind {
  /// A new handle over `value`, with an identity no other handle shares.
  #[must_use]
  pub fn new(value: impl Into<Value>) -> Self {
    Self {
      cell: SharedCell::new(value.into()),
    }
  }

  /// The value this handle binds.
  #[must_use]
  pub fn value(&self) -> &Value {
    &self.cell.payload
  }

  /// This handle's identity.
  pub(crate) fn id(&self) -> BindingId {
    self.cell.id
  }
}

/// A list of values bound once, however many `IN` predicates reference it.
///
/// The list-shaped twin of [`SharedBind`], with the same identity rule. An
/// empty handle binds nothing and renders the constant an empty `in_list`
/// already does: `1 = 0`, or `1 = 1` when negated.
///
/// ```
/// # use toolu_orm_core::{column::Text, dialect::Dialect, expr::SharedBindList};
/// # use toolu_orm_core::query_column::{Column, SharedOps};
/// const SRC: Column<Text> = Column::new("edges", "src_id");
/// const DST: Column<Text> = Column::new("edges", "dst_id");
///
/// let files = SharedBindList::new(["a.rs", "b.rs"]);
/// let (sql, params) = SRC
///   .in_shared(&files)
///   .or(DST.in_shared(&files))
///   .to_sql_fragment_for(1, Dialect::Sqlite);
/// assert_eq!(
///   sql,
///   r#"("edges"."src_id" IN (?1, ?2) OR "edges"."dst_id" IN (?1, ?2))"#
/// );
/// assert_eq!(params.len(), 2);
/// ```
pub struct SharedBindList {
  cell: Arc<SharedCell<Vec<Value>>>,
}

/// Cloning shares the binding; see [`SharedBind`]'s own `Clone`.
impl Clone for SharedBindList {
  fn clone(&self) -> Self {
    Self {
      cell: Arc::clone(&self.cell),
    }
  }
}

impl fmt::Debug for SharedBindList {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("SharedBindList")
      .field("id", &self.cell.id)
      .field("values", &self.cell.payload)
      .finish()
  }
}

impl SharedBindList {
  /// A new handle over `values`, with an identity no other handle shares.
  ///
  /// Takes anything iterable of anything convertible, so a `Vec<Value>`, a
  /// `Vec<String>` and a `["a", "b"]` array all work without the caller
  /// mapping first. A `&[&str]` does not: iterating a slice yields `&&str`,
  /// which is not `Into<Value>` — pass `slice.iter().copied()`.
  #[must_use]
  pub fn new<V: Into<Value>>(values: impl IntoIterator<Item = V>) -> Self {
    Self {
      cell: SharedCell::new(values.into_iter().map(Into::into).collect()),
    }
  }

  /// The values this handle binds, in placeholder order.
  #[must_use]
  pub fn values(&self) -> &[Value] {
    &self.cell.payload
  }

  /// How many values the handle binds.
  #[must_use]
  pub fn len(&self) -> usize {
    self.cell.payload.len()
  }

  /// Whether the handle binds nothing.
  #[must_use]
  pub fn is_empty(&self) -> bool {
    self.cell.payload.is_empty()
  }

  /// This handle's identity.
  pub(crate) fn id(&self) -> BindingId {
    self.cell.id
  }
}
