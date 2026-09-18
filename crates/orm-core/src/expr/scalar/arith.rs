//! Arithmetic and string concatenation between two scalars.

use std::ops::{Add, Div, Mul, Sub};

use super::types::{Scalar, ScalarKind};

impl Scalar {
  /// `(self || rhs)` — string concatenation, spelled the same on SQLite and
  /// Postgres.
  ///
  /// The arithmetic operators are the standard ones (`+`, `-`, `*`, `/`);
  /// `||` has no Rust operator, so it is a method.
  #[must_use]
  pub fn concat(self, rhs: Scalar) -> Self {
    self.arith("||", rhs)
  }

  /// Every operand pair is parenthesized, so a nested term keeps its
  /// precedence whatever it is dropped into.
  fn arith(self, op: &'static str, rhs: Scalar) -> Self {
    Self::from_kind(ScalarKind::Arith {
      left: Box::new(self),
      op,
      right: Box::new(rhs),
    })
  }
}

/// `(self + rhs)`.
impl Add for Scalar {
  type Output = Scalar;

  fn add(self, rhs: Scalar) -> Scalar {
    self.arith("+", rhs)
  }
}

/// `(self - rhs)`.
impl Sub for Scalar {
  type Output = Scalar;

  fn sub(self, rhs: Scalar) -> Scalar {
    self.arith("-", rhs)
  }
}

/// `(self * rhs)`.
impl Mul for Scalar {
  type Output = Scalar;

  fn mul(self, rhs: Scalar) -> Scalar {
    self.arith("*", rhs)
  }
}

/// `(self / rhs)`.
///
/// Division by zero is left to the engine: SQLite yields `NULL`, Postgres
/// raises SQLSTATE 22012 as a driver error.
impl Div for Scalar {
  type Output = Scalar;

  fn div(self, rhs: Scalar) -> Scalar {
    self.arith("/", rhs)
  }
}
