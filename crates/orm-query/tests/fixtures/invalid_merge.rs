use super::merge::{batch, row, ID, LABEL};
use toolu_orm_core::{alias::TableRef, column::Integer, query_column::Column, value::Value};
use toolu_orm_query::merge::{Matched, MergeBuilder, NotMatched};

/// Malformed batches for preflight checks against real seeded tables.
/// # Errors
/// Propagates construction failures for the function-target fixture.
pub fn cases() -> Result<Vec<MergeBuilder>, Box<dyn std::error::Error>> {
  let missing: Column<Integer> = Column::new("items", "missing");
  Ok(vec![
    batch(),
    batch().columns(&[]).row(row(1, "a", 1)),
    batch().keys(&[]).row(row(1, "a", 1)),
    batch().columns(&[&ID, &ID]).row(vec![1.into(), 1.into()]),
    batch().keys(&[&ID, &ID]).row(row(1, "a", 1)),
    batch().keys(&[&missing]).row(row(1, "a", 1)),
    batch().row(vec![1.into()]),
    batch().row(vec![Value::Null, "a".into(), 1.into()]),
    batch().row(vec![Value::Real(1.0), "a".into(), 1.into()]),
    batch().row(vec![Value::Blob(vec![1]), "a".into(), 1.into()]),
    batch().row(vec![Value::TimestampEpoch(1), "a".into(), 1.into()]),
    batch()
      .row(row(1, "a", 1))
      .row(vec!["2".into(), "b".into(), 2.into()]),
    batch()
      .when_matched(Matched::DoNothing)
      .when_not_matched(NotMatched::DoNothing)
      .row(row(1, "a", 1)),
    MergeBuilder::new("items")
      .columns(&[&ID, &LABEL])
      .keys(&[&ID])
      .row(vec![1.into(), "a".into()]),
    MergeBuilder::new("items")
      .columns(&[&ID])
      .keys(&[&ID])
      .row(vec![1.into()])
      .when_matched(Matched::Update)
      .when_not_matched(NotMatched::Insert),
    MergeBuilder::into_table(TableRef::function("items", vec![])?)
      .columns(&[&ID, &LABEL])
      .keys(&[&ID])
      .row(vec![1.into(), "a".into()])
      .when_matched(Matched::Update)
      .when_not_matched(NotMatched::Insert),
  ])
}
