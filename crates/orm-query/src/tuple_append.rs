//! Compile-time tuple growth for relational query result types.
//!
//! # Public API
//!
//! - [`TupleAppend`]

/// Append a value to a tuple, producing a tuple one element longer.
pub trait TupleAppend<T> {
  /// Tuple type after appending `T`.
  type Output;
  /// Append `value` to `self`.
  fn append(self, value: T) -> Self::Output;
}

macro_rules! impl_tuple_append_for {
  (($($T:ident),+), ($($idx:tt),+)) => {
    impl<$($T,)* Appended> TupleAppend<Appended> for ($($T,)+) {
      type Output = ($($T,)+ Appended);

      fn append(self, value: Appended) -> Self::Output {
        ($(self.$idx,)+ value)
      }
    }
  };
}

impl_tuple_append_for!((A), (0));
impl_tuple_append_for!((A, B), (0, 1));
impl_tuple_append_for!((A, B, C), (0, 1, 2));
impl_tuple_append_for!((A, B, C, D), (0, 1, 2, 3));
impl_tuple_append_for!((A, B, C, D, E), (0, 1, 2, 3, 4));
impl_tuple_append_for!((A, B, C, D, E, F), (0, 1, 2, 3, 4, 5));
impl_tuple_append_for!((A, B, C, D, E, F, G), (0, 1, 2, 3, 4, 5, 6));
impl_tuple_append_for!((A, B, C, D, E, F, G, H), (0, 1, 2, 3, 4, 5, 6, 7));
