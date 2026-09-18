//! Column-to-column predicates: `"a"."x" <op> "b"."y"`, binding nothing.

use crate::expr::{Expr, JoinCondition, Scalar};
use crate::query_column::Column;

use super::aliased_column::AliasedColumn;
use super::qualified::QualifiedColumn;

fn compare<L, R>(left: &L, op: &'static str, right: &R) -> JoinCondition
where
  L: QualifiedColumn + ?Sized,
  R: QualifiedColumn + ?Sized,
{
  JoinCondition::on(Expr::compare(
    Scalar::sql(left.qualified()),
    op,
    Scalar::sql(right.qualified()),
  ))
}

/// The six comparisons, generated for both column kinds so either side of a
/// join predicate may be plain or aliased.
macro_rules! impl_column_comparisons {
  ($($kind:ident),+ $(,)?) => {
    $(
      impl<T> $kind<T> {
        /// `self = other`.
        pub fn equals<C: QualifiedColumn + ?Sized>(&self, other: &C) -> JoinCondition {
          compare(self, "=", other)
        }

        /// `self != other`.
        pub fn not_equals<C: QualifiedColumn + ?Sized>(&self, other: &C) -> JoinCondition {
          compare(self, "!=", other)
        }

        /// `self < other`.
        pub fn less_than<C: QualifiedColumn + ?Sized>(&self, other: &C) -> JoinCondition {
          compare(self, "<", other)
        }

        /// `self <= other`.
        pub fn less_or_equal<C: QualifiedColumn + ?Sized>(&self, other: &C) -> JoinCondition {
          compare(self, "<=", other)
        }

        /// `self > other`.
        pub fn greater_than<C: QualifiedColumn + ?Sized>(&self, other: &C) -> JoinCondition {
          compare(self, ">", other)
        }

        /// `self >= other`.
        pub fn greater_or_equal<C: QualifiedColumn + ?Sized>(&self, other: &C) -> JoinCondition {
          compare(self, ">=", other)
        }
      }
    )+
  };
}

impl_column_comparisons!(Column, AliasedColumn);
