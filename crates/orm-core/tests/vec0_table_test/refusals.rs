//! What `build` refuses, and why quoting is not an option here.

use toolu_orm_core::column::VectorElement;
use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::vec0::{DistanceMetric, Vec0Table};

#[test]
fn a_name_vec0_cannot_read_is_refused_instead_of_quoted() {
  let error = Vec0Table::new("hostile\"); DROP TABLE users; --")
    .vector("embedding", VectorElement::Float, 4)
    .build()
    .expect_err("expected the table name to be refused");
  assert!(
    matches!(&error, DbCoreError::InvalidVec0Identifier { context, .. } if *context == "table name"),
    "wrong error: {error}"
  );

  let error = Vec0Table::new("memory_vec")
    .vector("embed\"ding", VectorElement::Float, 4)
    .build()
    .expect_err("expected the column name to be refused");
  assert!(
    matches!(&error, DbCoreError::InvalidVec0Identifier { context, ident }
      if *context == "column name" && ident == "embed\"ding"),
    "wrong error: {error}"
  );
}

/// Leading digits and underscores are outside `vec0`'s identifier rule too,
/// even though SQLite itself would accept them quoted.
#[test]
fn an_identifier_must_start_with_a_letter() {
  for name in ["_private", "1st", "", "with space", "dotted.name"] {
    assert!(
      Vec0Table::new(name)
        .vector("embedding", VectorElement::Float, 4)
        .build()
        .is_err(),
      "accepted the table name {name:?}"
    );
  }
  assert!(Vec0Table::new("memory_vec2")
    .vector("embed_ding", VectorElement::Float, 4)
    .build()
    .is_ok());
}

#[test]
fn a_bit_vector_cannot_carry_a_distance_metric() {
  let error = Vec0Table::new("bits")
    .vector_metric(
      "embedding",
      VectorElement::Bit,
      1024,
      DistanceMetric::Cosine,
    )
    .build()
    .expect_err("expected the bit vector to be refused");
  assert!(
    matches!(&error, DbCoreError::Vec0BitDistanceMetric { column } if column == "embedding"),
    "wrong error: {error}"
  );
  assert!(Vec0Table::new("bits")
    .vector("embedding", VectorElement::Bit, 1024)
    .build()
    .is_ok());
}
