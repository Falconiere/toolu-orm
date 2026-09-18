//! Table-valued `FROM` and `JOIN` sources: where their bound arguments land
//! among the statement's other placeholders.

use toolu_orm_core::alias::TableRef;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::expr::Scalar;
use toolu_orm_core::query_column::CommonOps;
use toolu_orm_core::value::Value;
use toolu_orm_query::select::SelectBuilder;

use crate::fixtures::{seeds_source, SEED_VALUE, SYMBOL_ID};

fn text(value: &str) -> Value {
  Value::Text(value.to_owned())
}

/// The select list renders before `FROM`, so a binding projection takes `?1`
/// and the source's argument takes `?2`.
#[test]
fn a_from_function_binds_after_the_select_list() {
  let seeds = seeds_source();
  let (sql, params) = SelectBuilder::from_table(&seeds)
    .column_scalar(Scalar::bind("tag"), "tag")
    .column_as(&seeds.column(&SEED_VALUE), "id")
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"SELECT ?1 AS "tag", "seeds"."value" AS "id" FROM json_each(?2) AS "seeds""#
  );
  assert_eq!(params, vec![text("tag"), text(r#"["a","b"]"#)]);
}

#[test]
fn the_same_source_numbers_with_dollar_placeholders_on_postgres() {
  let seeds = seeds_source();
  let (sql, params) = SelectBuilder::from_table(&seeds)
    .column_scalar(Scalar::bind("tag"), "tag")
    .to_sql_for(Dialect::Postgres);

  assert_eq!(sql, r#"SELECT $1 AS "tag" FROM json_each($2) AS "seeds""#);
  assert_eq!(params, vec![text("tag"), text(r#"["a","b"]"#)]);
}

/// A joined source is written before its `ON`, so it binds before it too.
#[test]
fn a_joined_function_binds_before_its_own_on_clause() {
  let seeds = seeds_source();
  let (sql, params) = SelectBuilder::new("code_symbols")
    .columns_qualified(&[&SYMBOL_ID])
    .join(&seeds, SYMBOL_ID.equals(&seeds.column(&SEED_VALUE)))
    .filter(SYMBOL_ID.ne("c9"))
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"SELECT "code_symbols"."id" FROM "code_symbols" "#.to_owned()
      + r#"INNER JOIN json_each(?1) AS "seeds" "#
      + r#"ON "code_symbols"."id" = "seeds"."value" "#
      + r#"WHERE "code_symbols"."id" != ?2"#
  );
  assert_eq!(params, vec![text(r#"["a","b"]"#), text("c9")]);
}

/// The `rebuild_copy.rs` shape: a pragma function reading a bound table name.
#[test]
fn a_pragma_function_source_binds_its_table_name() {
  let info = TableRef::function("pragma_table_info", vec![text("edges")])
    .expect("valid name")
    .with_alias("info");
  let (sql, params) = SelectBuilder::from_table(info)
    .columns_raw(&["name"])
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(sql, r#"SELECT "name" FROM pragma_table_info(?1) AS "info""#);
  assert_eq!(params, vec![text("edges")]);
}

/// The count and existence forms render no select list, so the source's
/// argument is the statement's *first* placeholder there.
#[test]
fn the_count_and_exists_forms_bind_the_source_first() {
  let seeds = seeds_source();
  let counted = SelectBuilder::from_table(&seeds)
    .filter(seeds.column(&SEED_VALUE).ne("x"))
    .to_count_sql_for(Dialect::Sqlite);
  let exists = SelectBuilder::from_table(&seeds).to_exists_sql_for(Dialect::Sqlite);

  assert_eq!(
    counted.0,
    r#"SELECT COUNT(*) FROM json_each(?1) AS "seeds" WHERE "seeds"."value" != ?2"#
  );
  assert_eq!(counted.1, vec![text(r#"["a","b"]"#), text("x")]);
  assert_eq!(
    exists.0,
    r#"SELECT EXISTS(SELECT 1 FROM json_each(?1) AS "seeds")"#
  );
  assert_eq!(exists.1, vec![text(r#"["a","b"]"#)]);
}

/// The name is syntax, so it is validated rather than escaped; nothing that is
/// not a bare identifier ever reaches a builder.
#[test]
fn a_hostile_function_name_never_reaches_a_statement() {
  let outcome = TableRef::function("edges); DROP TABLE edges; --", Vec::new()).err();

  assert!(
    matches!(outcome, Some(DbCoreError::InvalidTableFunction { .. })),
    "got: {outcome:?}"
  );
}
