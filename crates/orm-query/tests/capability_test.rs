//! Capability checks operate on structured state, independent of driver features.

use toolu_orm_connection::{require_capabilities, Capability, DbError};
use toolu_orm_core::{column::Integer, dialect::Dialect, query_column::Column};
use toolu_orm_query::{
  delete::DeleteBuilder,
  insert::{InsertBuilder, OnConflict},
  update::UpdateBuilder,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const ID: Column<Integer> = Column::new("items", "id");

fn refused(result: Result<(), DbError>, expected: Capability) -> TestResult {
  let error = result
    .err()
    .ok_or("Lance must refuse the requested guarantee")?;
  assert!(
    matches!(error, DbError::UnsupportedCapability { backend: Dialect::Lance, capability } if capability == expected)
  );
  let message = error.to_string();
  assert!(message.contains("lance"));
  assert!(message.contains(expected.as_str()));
  assert!(message.contains(expected.alternative()));
  assert!(!expected.alternative().is_empty());
  Ok(())
}

#[test]
fn requirements_are_named_and_runtime_selected() -> TestResult {
  let capabilities = [
    Capability::OnConflict,
    Capability::DmlReturning,
    Capability::PrimaryKey,
    Capability::UniqueConstraint,
    Capability::UniqueIndex,
    Capability::ForeignKey,
    Capability::NotNull,
    Capability::CheckConstraint,
    Capability::MultiStatementTransaction,
  ];
  for capability in capabilities {
    refused(
      require_capabilities(Dialect::Lance, &[capability]),
      capability,
    )?;
    for dialect in [Dialect::Sqlite, Dialect::Postgres] {
      assert!(require_capabilities(dialect, &[capability]).is_ok());
    }
  }
  assert!(require_capabilities(Dialect::Lance, &[]).is_ok());
  Ok(())
}

#[test]
fn conflict_modes_and_returning_use_structured_state() -> TestResult {
  let builders = [
    InsertBuilder::new("items").or_ignore(),
    InsertBuilder::new("items").or_replace(),
    InsertBuilder::new("items").on_conflict(OnConflict::column(&ID)),
    InsertBuilder::new("items").on_conflict(OnConflict::column(&ID).set(&ID, 2_i64)),
    InsertBuilder::new("items").or_ignore().returning(&ID),
  ];
  for builder in builders {
    refused(builder.validate_for(Dialect::Lance), Capability::OnConflict)?;
    assert!(builder.validate_for(Dialect::Sqlite).is_ok());
    assert!(builder.validate_for(Dialect::Postgres).is_ok());
  }
  for dialect in [Dialect::Sqlite, Dialect::Postgres, Dialect::Lance] {
    let results = [
      InsertBuilder::new("items")
        .returning(&ID)
        .validate_for(dialect),
      UpdateBuilder::new("items")
        .returning(&ID)
        .validate_for(dialect),
      DeleteBuilder::new("items")
        .returning(&ID)
        .validate_for(dialect),
    ];
    for result in results {
      if dialect == Dialect::Lance {
        refused(result, Capability::DmlReturning)?;
      } else {
        assert!(result.is_ok());
      }
    }
  }
  Ok(())
}

#[test]
fn names_values_and_inactive_conflict_hints_do_not_trigger_refusal() {
  assert!(InsertBuilder::new("ON CONFLICT RETURNING")
    .set(&ID, "BEGIN; ON CONFLICT; RETURNING")
    .conflict_columns(&["id"])
    .validate_for(Dialect::Lance)
    .is_ok());
  assert!(UpdateBuilder::new("RETURNING")
    .validate_for(Dialect::Lance)
    .is_ok());
  assert!(DeleteBuilder::new("ON CONFLICT")
    .validate_for(Dialect::Lance)
    .is_ok());
}
