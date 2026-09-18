//! Subqueries in predicate and scalar position, and the set-based `DELETE`
//! they make possible — every one nesting a real `SelectBuilder`.

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::{Expr, Scalar, SelectSource};
use toolu_orm_core::query_column::CommonOps;
use toolu_orm_core::value::Value;
use toolu_orm_query::delete::DeleteBuilder;
use toolu_orm_query::select::SelectBuilder;

use crate::fixtures::{by_repo, ids_of_repo_path, ITEM_ID, ITEM_OWNER_ID, OWNER_ID, VEC_SYMBOL_ID};

fn text(value: &str) -> Value {
  Value::Text(value.to_owned())
}

/// The inner statement's placeholders continue the outer statement's count.
#[test]
fn an_in_subquery_continues_the_outer_bind_numbering() {
  let (sql, params) = SelectBuilder::new("items")
    .columns_qualified(&[&ITEM_ID])
    .filter(ITEM_ID.ne("i9"))
    .filter(Scalar::col(&ITEM_OWNER_ID).in_subquery(owner_ids()))
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"SELECT "items"."id" FROM "items" WHERE "items"."id" != ?1 "#.to_owned()
      + r#"AND "items"."owner_id" IN (SELECT "owners"."id" FROM "owners" "#
      + r#"WHERE "owners"."id" != ?2)"#
  );
  assert_eq!(params, vec![text("i9"), text("o9")]);
}

#[test]
fn the_same_predicate_numbers_with_dollar_placeholders_on_postgres() {
  let (sql, params) = SelectBuilder::new("items")
    .columns_qualified(&[&ITEM_ID])
    .filter(ITEM_ID.ne("i9"))
    .filter(Scalar::col(&ITEM_OWNER_ID).in_subquery(owner_ids()))
    .to_sql_for(Dialect::Postgres);

  assert!(sql.contains(r#""items"."id" != $1"#), "got: {sql}");
  assert!(sql.contains(r#""owners"."id" != $2)"#), "got: {sql}");
  assert_eq!(params, vec![text("i9"), text("o9")]);
}

#[test]
fn not_in_renders_the_negated_keyword() {
  let (sql, _) = SelectBuilder::new("items")
    .columns_qualified(&[&ITEM_ID])
    .filter(Scalar::col(&ITEM_OWNER_ID).not_in_subquery(owner_ids()))
    .to_sql_for(Dialect::Sqlite);

  assert!(
    sql.contains(r#""items"."owner_id" NOT IN (SELECT "#),
    "got: {sql}"
  );
}

/// A correlated `EXISTS`: the inner statement names an outer column, which is
/// just a qualified reference and needs nothing from the builder.
#[test]
fn a_correlated_exists_names_the_outer_column_inside_the_subquery() {
  let (sql, params) = SelectBuilder::new("items")
    .columns_qualified(&[&ITEM_ID])
    .filter(Expr::not_exists(owners_of_item()))
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"SELECT "items"."id" FROM "items" WHERE NOT EXISTS (SELECT 1 AS "one" "#.to_owned()
      + r#"FROM "owners" WHERE "owners"."id" = "items"."owner_id")"#
  );
  assert!(params.is_empty());
}

#[test]
fn the_positive_form_drops_the_not() {
  let (sql, _) = SelectBuilder::new("items")
    .columns_qualified(&[&ITEM_ID])
    .filter(Expr::exists(owners_of_item()))
    .to_sql_for(Dialect::Sqlite);

  assert!(
    sql.contains(r#"WHERE EXISTS (SELECT 1 AS "one""#),
    "got: {sql}"
  );
}

/// A scalar subquery in projection position, parenthesised and aliased.
#[test]
fn a_scalar_subquery_projects_a_correlated_count() {
  let (sql, params) = SelectBuilder::new("items")
    .columns_qualified(&[&ITEM_ID])
    .column_scalar(Scalar::subquery(owner_count_of_item()), "owner_rows")
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"SELECT "items"."id", (SELECT COUNT(*) AS "n" FROM "owners" "#.to_owned()
      + r#"WHERE "owners"."id" = "items"."owner_id") AS "owner_rows" FROM "items""#
  );
  assert!(params.is_empty());
}

/// The set-based DML the issue asks for: the owner-id set never reaches Rust,
/// so the parameter vector holds only the subquery's own two values.
#[test]
fn a_set_based_delete_binds_only_the_subquerys_own_values() {
  let (sql, params) = DeleteBuilder::new("code_vec")
    .filter(Scalar::col(&VEC_SYMBOL_ID).in_subquery(ids_of_repo_path("r1", "src/a.rs")))
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"DELETE FROM "code_vec" WHERE "code_vec"."symbol_id" IN ("#.to_owned()
      + r#"SELECT "code_symbols"."id" FROM "code_symbols" "#
      + r#"WHERE "code_symbols"."repo" = ?1 AND "code_symbols"."path" = ?2)"#
  );
  assert_eq!(params, vec![text("r1"), text("src/a.rs")]);
}

/// A subquery carries its own `WITH` prefix and `UNION` arms whole, so the
/// composition nests to any depth with one numbering rule.
#[test]
fn a_compound_subquery_nests_with_all_of_its_own_clauses() {
  let (sql, params) = SelectBuilder::new("code_vec")
    .columns_raw(&["symbol_id"])
    .filter(Scalar::col(&VEC_SYMBOL_ID).in_subquery(by_repo("r1").union(by_repo("r2"))))
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"SELECT "symbol_id" FROM "code_vec" WHERE "code_vec"."symbol_id" IN ("#.to_owned()
      + r#"SELECT "code_symbols"."id" FROM "code_symbols" WHERE "code_symbols"."repo" = ?1 "#
      + r#"UNION SELECT "code_symbols"."id" FROM "code_symbols" "#
      + r#"WHERE "code_symbols"."repo" = ?2)"#
  );
  assert_eq!(params, vec![text("r1"), text("r2")]);
}

/// Every id in the whole statement, from every nesting level, is used exactly
/// once and in order: `1..=params.len()`, no repeat and no gap.
#[test]
fn every_placeholder_index_is_used_exactly_once_in_order() {
  let (sql, params) = SelectBuilder::new("items")
    .columns_qualified(&[&ITEM_ID])
    .filter(ITEM_ID.ne("i9"))
    .filter(Scalar::col(&ITEM_OWNER_ID).in_subquery(owner_ids()))
    .limit(5)
    .to_sql_for(Dialect::Sqlite);

  let indices: Vec<usize> = sql
    .match_indices('?')
    .filter_map(|(at, _)| sql[at + 1..].split(|c: char| !c.is_ascii_digit()).next())
    .filter_map(|digits| digits.parse().ok())
    .collect();

  assert_eq!(indices, (1..=params.len()).collect::<Vec<usize>>());
}

fn owner_ids() -> SelectBuilder {
  SelectBuilder::new("owners")
    .columns_qualified(&[&OWNER_ID])
    .filter(OWNER_ID.ne("o9"))
}

fn owners_of_item() -> SelectBuilder {
  SelectBuilder::new("owners")
    .column_expr("1", "one")
    .filter(OWNER_ID.equals(&ITEM_OWNER_ID).into())
}

fn owner_count_of_item() -> SelectBuilder {
  SelectBuilder::new("owners")
    .column_scalar(Scalar::count_star(), "n")
    .filter(OWNER_ID.equals(&ITEM_OWNER_ID).into())
}

/// The offset frame that `SelectSource::to_select_sql_for` opens returns
/// **only** the values this statement binds: the stand-ins for what the caller
/// already emitted never leave the function, whatever `start` is.
#[test]
fn a_nested_statement_returns_only_its_own_values_at_any_offset() {
  let inner = owner_ids();

  for start in [1, 2, 7] {
    let (sql, params) = inner.to_select_sql_for(start, Dialect::Sqlite);

    assert_eq!(
      sql,
      format!(r#"SELECT "owners"."id" FROM "owners" WHERE "owners"."id" != ?{start}"#)
    );
    assert_eq!(params, vec![text("o9")], "start = {start}");
  }
}
