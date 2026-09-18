//! Aggregate scalar nodes: the two shapes `Scalar::func` cannot express, the
//! four that it could, and how each one numbers a bound argument.

use toolu_orm_core::column::{Integer, Text};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::Scalar;
use toolu_orm_core::query_column::Column;
use toolu_orm_core::value::Value;

const PATH: Column<Text> = Column::new("source_files", "path");
const SIZE: Column<Integer> = Column::new("source_files", "size_bytes");

#[test]
fn count_star_renders_the_star_and_binds_nothing() {
  let (sql, params) = Scalar::count_star().to_sql_fragment_for(1, Dialect::Sqlite);

  assert_eq!(sql, "COUNT(*)");
  assert!(params.is_empty());
}

#[test]
fn count_distinct_puts_the_keyword_inside_the_parentheses() {
  let (sql, params) =
    Scalar::count_distinct(Scalar::col(&PATH)).to_sql_fragment_for(1, Dialect::Sqlite);

  assert_eq!(sql, r#"COUNT(DISTINCT "source_files"."path")"#);
  assert!(params.is_empty());
}

#[test]
fn a_plain_count_argument_carries_no_distinct_keyword() {
  let (sql, _) = Scalar::count(Scalar::col(&PATH)).to_sql_fragment_for(1, Dialect::Sqlite);

  assert_eq!(sql, r#"COUNT("source_files"."path")"#);
}

#[test]
fn every_aggregate_renders_its_own_uppercase_name() {
  let cases = [
    (Scalar::sum(Scalar::col(&SIZE)), "SUM"),
    (Scalar::max(Scalar::col(&SIZE)), "MAX"),
    (Scalar::min(Scalar::col(&SIZE)), "MIN"),
    (Scalar::avg(Scalar::col(&SIZE)), "AVG"),
  ];

  for (scalar, name) in cases {
    let (sql, _) = scalar.to_sql_fragment_for(1, Dialect::Sqlite);
    assert_eq!(sql, format!(r#"{name}("source_files"."size_bytes")"#));
  }
}

/// The whole point of routing aggregates through `ScalarKind`: the argument is
/// rendered by `render_scalar`, so it takes its index from the live parameter
/// count rather than from `1`.
#[test]
fn an_aggregate_over_a_bound_value_numbers_from_the_offset() {
  let (sql, params) = Scalar::sum(Scalar::bind(2i64)).to_sql_fragment_for(4, Dialect::Sqlite);

  assert_eq!(sql, "SUM(?4)");
  assert_eq!(params, vec![Value::Integer(2)]);
}

#[test]
fn the_same_aggregate_renders_dollar_placeholders_on_postgres() {
  let (sql, params) = Scalar::sum(Scalar::bind(2i64)).to_sql_fragment_for(4, Dialect::Postgres);

  assert_eq!(sql, "SUM($4)");
  assert_eq!(params.len(), 1);
}

/// An aggregate is a `Scalar`, so it composes with arithmetic and with the
/// comparisons that build a `HAVING` predicate.
#[test]
fn aggregates_compose_with_arithmetic_and_comparisons() {
  let ratio = Scalar::sum(Scalar::col(&SIZE)) / Scalar::count_star();
  let (sql, params) = ratio.to_sql_fragment_for(1, Dialect::Sqlite);
  assert_eq!(sql, r#"(SUM("source_files"."size_bytes") / COUNT(*))"#);
  assert!(params.is_empty());

  let predicate = Scalar::count_star().gt(Scalar::bind(1i64));
  let (sql, params) = predicate.to_sql_fragment_for(3, Dialect::Sqlite);
  assert_eq!(sql, "COUNT(*) > ?3");
  assert_eq!(params, vec![Value::Integer(1)]);

  let ordered = Scalar::count_star().desc();
  assert_eq!(
    ordered.to_sql_fragment_for(1, Dialect::Sqlite).0,
    "COUNT(*) DESC"
  );
}

/// An aggregate can name a column of an *aliased* relation, whose base table
/// name SQL has hidden, through `AliasedColumn::scalar`.
#[test]
fn an_aggregate_can_name_a_column_of_an_aliased_relation() {
  use toolu_orm_core::alias::TableRef;

  let files = TableRef::aliased("source_files", "f");
  let aggregate = Scalar::count_distinct(files.column(&PATH).scalar());
  let (sql, _) = aggregate.to_sql_fragment_for(1, Dialect::Sqlite);

  assert_eq!(sql, r#"COUNT(DISTINCT "f"."path")"#);
}
