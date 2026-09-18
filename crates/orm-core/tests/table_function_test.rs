//! `TableRef` as a table-valued `FROM` source: the rendered call, where its
//! placeholders are numbered from, the alias, and the refusal of a name that is
//! not a bare identifier.
//!
//! Every assertion names its dialect, because this binary is listed by both the
//! default and the postgres lanes and `Dialect::CURRENT` differs between them.

use toolu_orm_core::alias::TableRef;
use toolu_orm_core::column::Text;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::query_column::Column;
use toolu_orm_core::value::Value;

/// `json_each` exposes a `value` column; naming it as a `Column` is how a
/// projection addresses the function's output.
const VALUE: Column<Text> = Column::new("json_each", "value");

fn seeds_json() -> Value {
  Value::Text(r#"["a","b"]"#.to_owned())
}

#[test]
fn a_function_source_renders_its_call_and_binds_its_argument() {
  let source = TableRef::function("json_each", vec![seeds_json()]).expect("valid name");

  let (sql, params) = source.to_sql_fragment_for(1, Dialect::Sqlite);

  assert_eq!(sql, "json_each(?1)");
  assert_eq!(params, vec![seeds_json()]);
}

/// The whole point of the fragment form: a `FROM` slot is numbered from what
/// the clauses rendered before it already emitted, not from 1.
#[test]
fn a_function_source_numbers_from_the_offset_it_is_given() {
  let source = TableRef::function("json_each", vec![seeds_json()]).expect("valid name");

  let (sqlite, params) = source.to_sql_fragment_for(4, Dialect::Sqlite);
  let (postgres, _) = source.to_sql_fragment_for(4, Dialect::Postgres);

  assert_eq!(sqlite, "json_each(?4)");
  assert_eq!(postgres, "json_each($4)");
  assert_eq!(params, vec![seeds_json()]);
}

#[test]
fn several_arguments_number_consecutively_from_the_offset() {
  let source = TableRef::function(
    "pragma_table_info",
    vec![
      Value::Text("edges".to_owned()),
      Value::Text("old".to_owned()),
    ],
  )
  .expect("valid name");

  let (sql, params) = source.to_sql_fragment_for(2, Dialect::Sqlite);

  assert_eq!(sql, "pragma_table_info(?2, ?3)");
  assert_eq!(
    params,
    vec![
      Value::Text("edges".to_owned()),
      Value::Text("old".to_owned())
    ]
  );
}

#[test]
fn a_zero_argument_function_renders_empty_parentheses_and_binds_nothing() {
  let source = TableRef::function("pragma_database_list", Vec::new()).expect("valid name");

  let (sql, params) = source.to_sql_fragment_for(1, Dialect::Sqlite);

  assert_eq!(sql, "pragma_database_list()");
  assert!(params.is_empty());
}

#[test]
fn an_alias_follows_the_call_and_qualifies_its_columns() {
  let source = TableRef::function("json_each", vec![seeds_json()])
    .expect("valid name")
    .with_alias("seeds");

  let (sql, _) = source.to_sql_fragment_for(1, Dialect::Sqlite);

  assert_eq!(sql, r#"json_each(?1) AS "seeds""#);
  assert_eq!(source.qualifier(), "seeds");
  assert_eq!(source.alias(), Some("seeds"));
  assert_eq!(source.table(), "json_each");
  assert_eq!(source.column(&VALUE).qualified(), r#""seeds"."value""#);
}

/// The regression guard for #111: every source that binds nothing renders
/// exactly the string it always rendered, through the new fragment path.
#[test]
fn a_relation_source_renders_exactly_what_it_always_did() {
  let plain = TableRef::new("code_symbols");
  let aliased = TableRef::aliased("code_symbols", "old");

  let (plain_sql, plain_params) = plain.to_sql_fragment_for(7, Dialect::Postgres);
  let (aliased_sql, aliased_params) = aliased.to_sql_fragment_for(7, Dialect::Postgres);

  assert_eq!(plain_sql, r#""code_symbols""#);
  assert_eq!(plain.to_sql(), r#""code_symbols""#);
  assert!(plain_params.is_empty());
  assert_eq!(aliased_sql, r#""code_symbols" AS "old""#);
  assert_eq!(aliased.to_sql(), r#""code_symbols" AS "old""#);
  assert!(aliased_params.is_empty());
}

/// `with_alias` is the companion of `aliased` for the sources that have no
/// two-argument constructor; on a relation the two agree.
#[test]
fn with_alias_agrees_with_the_two_argument_constructor() {
  assert_eq!(
    TableRef::new("code_symbols").with_alias("old"),
    TableRef::aliased("code_symbols", "old")
  );
}

/// A CTE is referenced by a plain identifier, so it needs no source of its own.
#[test]
fn a_cte_name_is_an_ordinary_relation_source() {
  let walk = TableRef::new("walk").with_alias("w");

  assert_eq!(walk.to_sql(), r#""walk" AS "w""#);
  assert_eq!(walk.qualifier(), "w");
}

#[test]
fn a_function_name_that_is_not_a_bare_identifier_is_refused() {
  for hostile in ["drop table users; --", "", "1bad", "json each", "json.each"] {
    let outcome = TableRef::function(hostile, vec![seeds_json()]).err();
    assert!(
      matches!(&outcome, Some(DbCoreError::InvalidTableFunction { name }) if name == hostile),
      "{hostile:?} should have been refused, got {outcome:?}"
    );
  }
}

/// The name is validated; the *alias* is quoted, with the embedded quote
/// doubled, so an injection attempt becomes ordinary characters of a name.
#[test]
fn a_hostile_alias_is_quote_doubled_rather_than_refused() {
  let source = TableRef::function("json_each", vec![seeds_json()])
    .expect("valid name")
    .with_alias(r#"x"; DROP TABLE edges; --"#);

  let (sql, _) = source.to_sql_fragment_for(1, Dialect::Sqlite);

  assert_eq!(
    sql, r#"json_each(?1) AS "x""; DROP TABLE edges; --""#,
    "the only unpaired quotes must be the delimiters"
  );
}
