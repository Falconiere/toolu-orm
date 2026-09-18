//! `UNION` / `UNION ALL`: what the arms contribute, what the tail bounds, how
//! nesting flattens, and how a compound is counted.

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::OrderBy;
use toolu_orm_core::value::Value;
use toolu_orm_query::select::SelectBuilder;

use crate::fixtures::{by_path, by_repo, SYMBOL_ID};

fn text(value: &str) -> Value {
  Value::Text(value.to_owned())
}

/// The right arm continues the left arm's bind count. Restarting it at `?1`
/// would send the wrong value to the wrong branch with no error anywhere — so
/// this is asserted as one exact `(sql, params)` pair.
#[test]
fn a_union_arm_continues_the_left_arms_bind_numbering() {
  let (sql, params) = by_repo("r1")
    .union(by_path("src/a.rs"))
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"SELECT "code_symbols"."id" FROM "code_symbols" WHERE "code_symbols"."repo" = ?1 "#
      .to_owned()
      + r#"UNION SELECT "code_symbols"."id" FROM "code_symbols" "#
      + r#"WHERE "code_symbols"."path" = ?2"#
  );
  assert_eq!(params, vec![text("r1"), text("src/a.rs")]);
}

#[test]
fn the_same_compound_numbers_with_dollar_placeholders_on_postgres() {
  let (sql, params) = by_repo("r1")
    .union(by_path("src/a.rs"))
    .to_sql_for(Dialect::Postgres);

  assert_eq!(
    sql,
    r#"SELECT "code_symbols"."id" FROM "code_symbols" WHERE "code_symbols"."repo" = $1 "#
      .to_owned()
      + r#"UNION SELECT "code_symbols"."id" FROM "code_symbols" "#
      + r#"WHERE "code_symbols"."path" = $2"#
  );
  assert_eq!(params, vec![text("r1"), text("src/a.rs")]);
}

#[test]
fn union_all_renders_the_all_keyword() {
  let (sql, _) = by_repo("r1")
    .union_all(by_path("src/a.rs"))
    .to_sql_for(Dialect::Sqlite);

  assert!(sql.contains(" UNION ALL SELECT "), "got: {sql}");
  assert!(!sql.contains(" UNION SELECT "), "got: {sql}");
}

/// A nested compound must flatten. Rendering only the arm's *core* would drop
/// the third branch entirely — a wrong result set with no error.
#[test]
fn a_nested_compound_flattens_to_the_same_sql_as_a_chained_one() {
  let nested = by_repo("r1")
    .union(by_path("src/a.rs").union(by_path("src/b.rs")))
    .to_sql_for(Dialect::Sqlite);
  let chained = by_repo("r1")
    .union(by_path("src/a.rs"))
    .union(by_path("src/b.rs"))
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(nested, chained);
  assert_eq!(nested.0.matches(" UNION SELECT ").count(), 2);
  assert_eq!(
    nested.1,
    vec![text("r1"), text("src/a.rs"), text("src/b.rs")]
  );
}

/// `ORDER BY` / `LIMIT` / `OFFSET` belong to the whole compound, so they render
/// once, after the last arm — and the ordering names an *output* alias, which
/// is the only form Postgres accepts on a compound.
#[test]
fn the_tail_renders_after_the_last_arm_and_binds_last() {
  let (sql, params) = by_repo("r1")
    .union(by_path("src/a.rs"))
    .order_by(OrderBy::alias_asc("id"))
    .limit(2)
    .offset(1)
    .to_sql_for(Dialect::Sqlite);

  assert!(
    sql.ends_with(r#"ORDER BY "id" ASC LIMIT ?3 OFFSET ?4"#),
    "got: {sql}"
  );
  assert_eq!(
    params,
    vec![
      text("r1"),
      text("src/a.rs"),
      Value::Integer(2),
      Value::Integer(1)
    ]
  );
}

/// SQL has no place for a row window on an individual arm, so an arm that
/// carries one renders exactly as if it did not.
#[test]
fn an_arms_own_order_by_and_limit_are_not_rendered() {
  let bare = by_repo("r1")
    .union(by_path("src/a.rs"))
    .to_sql_for(Dialect::Sqlite);
  let decorated = by_repo("r1")
    .union(
      by_path("src/a.rs")
        .order_by(SYMBOL_ID.desc())
        .limit(1)
        .offset(5),
    )
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(decorated, bare);
}

/// A compound's rows are the merged set, so counting it needs the derived
/// table: `SELECT COUNT(*)` appended to the left arm would count that arm only.
#[test]
fn a_compound_count_wraps_the_whole_compound_in_the_derived_table() {
  let (sql, params) = by_repo("r1")
    .union(by_path("src/a.rs"))
    .order_by(OrderBy::alias_asc("id"))
    .limit(10)
    .to_count_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"SELECT COUNT(*) FROM (SELECT "code_symbols"."id" FROM "code_symbols" "#.to_owned()
      + r#"WHERE "code_symbols"."repo" = ?1 UNION SELECT "code_symbols"."id" "#
      + r#"FROM "code_symbols" WHERE "code_symbols"."path" = ?2) AS "toolu_count""#
  );
  assert_eq!(params, vec![text("r1"), text("src/a.rs")]);
}

/// Postgres *requires* the alias on a subquery in `FROM`; SQLite merely
/// tolerates it, so there is one rendering to reason about.
#[test]
fn the_compound_count_is_aliased_on_postgres_too() {
  let (sql, _) = by_repo("r1")
    .union(by_path("src/a.rs"))
    .to_count_sql_for(Dialect::Postgres);

  assert!(
    sql.starts_with("SELECT COUNT(*) FROM (SELECT "),
    "got: {sql}"
  );
  assert!(sql.ends_with(r#") AS "toolu_count""#), "got: {sql}");
  assert!(sql.contains(" UNION SELECT "), "got: {sql}");
}

/// `EXISTS` takes the compound whole — a compound select is a select
/// statement, so no derived table is needed.
#[test]
fn a_compound_exists_wraps_the_compound_not_a_derived_table() {
  let (sql, params) = by_repo("r1")
    .union(by_path("src/a.rs"))
    .to_exists_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"SELECT EXISTS(SELECT "code_symbols"."id" FROM "code_symbols" "#.to_owned()
      + r#"WHERE "code_symbols"."repo" = ?1 UNION SELECT "code_symbols"."id" "#
      + r#"FROM "code_symbols" WHERE "code_symbols"."path" = ?2)"#
  );
  assert_eq!(params, vec![text("r1"), text("src/a.rs")]);
}

/// The #109 backwards-compatibility contract, extended: a builder using none
/// of the new clauses renders exactly the count and existence SQL it always
/// rendered.
#[test]
fn a_builder_with_no_arms_renders_the_plain_count_and_exists_unchanged() {
  let plain = SelectBuilder::new("code_symbols").columns_raw(&["id"]);

  assert_eq!(
    plain.to_count_sql_for(Dialect::Sqlite).0,
    r#"SELECT COUNT(*) FROM "code_symbols""#
  );
  assert_eq!(
    plain.to_exists_sql_for(Dialect::Sqlite).0,
    r#"SELECT EXISTS(SELECT 1 FROM "code_symbols")"#
  );
}
