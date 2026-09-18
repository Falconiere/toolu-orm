//! `WITH` / `WITH RECURSIVE`: where the prefix sits, which keyword it uses,
//! and where its bodies' parameters land.

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::query_column::CommonOps;
use toolu_orm_core::value::Value;
use toolu_orm_query::select::{Cte, SelectBuilder};

use crate::fixtures::{by_path, by_repo, walk_rows, SYMBOL_ID};

fn text(value: &str) -> Value {
  Value::Text(value.to_owned())
}

#[test]
fn a_plain_cte_renders_before_select_and_binds_first() {
  let (sql, params) = SelectBuilder::from_table("hits")
    .columns_raw(&["id"])
    .with(Cte::new("hits", by_repo("r1")))
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"WITH "hits" AS (SELECT "code_symbols"."id" FROM "code_symbols" "#.to_owned()
      + r#"WHERE "code_symbols"."repo" = ?1) SELECT "id" FROM "hits""#
  );
  assert_eq!(params, vec![text("r1")]);
}

/// The prefix binds *before* the outer statement, so the outer `WHERE` takes
/// the higher index. Getting this backwards would swap two values silently.
#[test]
fn the_outer_statement_numbers_after_every_cte_body() {
  let (sql, params) = SelectBuilder::from_table("hits")
    .columns_raw(&["id"])
    .with(Cte::new("hits", by_repo("r1")))
    .filter(SYMBOL_ID.eq("c2"))
    .to_sql_for(Dialect::Sqlite);

  assert!(sql.contains(r#""code_symbols"."repo" = ?1"#), "got: {sql}");
  assert!(
    sql.ends_with(r#"WHERE "code_symbols"."id" = ?2"#),
    "got: {sql}"
  );
  assert_eq!(params, vec![text("r1"), text("c2")]);
}

#[test]
fn several_ctes_render_in_call_order_separated_by_commas() {
  let (sql, params) = SelectBuilder::from_table("a")
    .columns_raw(&["id"])
    .with(Cte::new("a", by_repo("r1")))
    .with(Cte::new("b", by_path("src/a.rs")))
    .to_sql_for(Dialect::Sqlite);

  assert!(sql.starts_with(r#"WITH "a" AS (SELECT "#), "got: {sql}");
  assert!(sql.contains(r#"?1), "b" AS (SELECT "#), "got: {sql}");
  assert_eq!(params, vec![text("r1"), text("src/a.rs")]);
}

#[test]
fn an_explicit_column_list_is_quoted_and_parenthesised() {
  let (sql, _) = walk_rows(2).to_sql_for(Dialect::Sqlite);

  assert!(
    sql.starts_with(r#"WITH RECURSIVE "walk"("kind", "id", "depth") AS ("#),
    "got: {sql}"
  );
}

/// One recursive member makes the whole prefix `WITH RECURSIVE`; with none it
/// stays plain `WITH`.
#[test]
fn the_recursive_keyword_appears_only_when_a_member_asks_for_it() {
  let plain = SelectBuilder::from_table("hits")
    .columns_raw(&["id"])
    .with(Cte::new("hits", by_repo("r1")))
    .to_sql_for(Dialect::Sqlite);
  let recursive = SelectBuilder::from_table("hits")
    .columns_raw(&["id"])
    .with(Cte::new("hits", by_repo("r1")).recursive())
    .to_sql_for(Dialect::Sqlite);

  assert!(
    plain.0.starts_with(r#"WITH "hits" AS ("#),
    "got: {}",
    plain.0
  );
  assert!(
    recursive.0.starts_with(r#"WITH RECURSIVE "hits" AS ("#),
    "got: {}",
    recursive.0
  );
}

/// The whole production shape from the issue, as one exact `(sql, params)`
/// pair: a recursive CTE whose body is a `UNION`, a bound seed filter, a bound
/// depth limit, then a grouped read of the CTE.
#[test]
fn the_recursive_walk_renders_the_issues_statement() {
  let (sql, params) = walk_rows(2).to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"WITH RECURSIVE "walk"("kind", "id", "depth") AS ("#.to_owned()
      + r#"SELECT "w"."kind" AS "kind", "w"."id" AS "id", CAST(0 AS BIGINT) AS "depth" "#
      + r#"FROM "walk_seeds" AS "w" WHERE "w"."batch" = ?1 "#
      + r#"UNION "#
      + r#"SELECT "e"."dst_kind" AS "kind", "e"."dst_id" AS "id", ("s"."depth" + 1) AS "depth" "#
      + r#"FROM "edges" AS "e" INNER JOIN "walk" AS "s" "#
      + r#"ON ("e"."src_kind" = "s"."kind" AND "e"."src_id" = "s"."id") "#
      + r#"WHERE "s"."depth" < ?2) "#
      + r#"SELECT "walk"."id" AS "id", MIN("walk"."depth") AS "depth" FROM "walk" "#
      + r#"GROUP BY "walk"."id" ORDER BY "walk"."id" ASC"#
  );
  assert_eq!(params, vec![text("b1"), Value::Integer(2)]);
}

#[test]
fn the_same_walk_numbers_with_dollar_placeholders_on_postgres() {
  let (sql, params) = walk_rows(2).to_sql_for(Dialect::Postgres);

  assert!(sql.contains(r#""w"."batch" = $1"#), "got: {sql}");
  assert!(sql.contains(r#""s"."depth" < $2"#), "got: {sql}");
  assert_eq!(params, vec![text("b1"), Value::Integer(2)]);
}

/// A `WITH` belongs to the whole statement, so an arm's CTEs are hoisted into
/// the one prefix rather than dropped — dropping them would leave the arm
/// naming a relation nothing declared.
#[test]
fn an_arms_cte_is_hoisted_into_the_single_with_prefix() {
  let arm = SelectBuilder::from_table("b")
    .columns_raw(&["id"])
    .with(Cte::new("b", by_path("src/a.rs")));
  let (sql, params) = SelectBuilder::from_table("a")
    .columns_raw(&["id"])
    .with(Cte::new("a", by_repo("r1")))
    .union(arm)
    .to_sql_for(Dialect::Sqlite);

  assert!(sql.starts_with(r#"WITH "a" AS ("#), "got: {sql}");
  assert!(sql.contains(r#", "b" AS ("#), "got: {sql}");
  assert_eq!(sql.matches("WITH ").count(), 1, "got: {sql}");
  assert!(
    sql.contains(r#"SELECT "id" FROM "a" UNION SELECT "id" FROM "b""#),
    "got: {sql}"
  );
  assert_eq!(params, vec![text("r1"), text("src/a.rs")]);
}

/// SQL puts `WITH` before the counted statement, not inside the derived table.
#[test]
fn the_with_prefix_stays_outside_the_count_wrap_and_the_exists() {
  let counted = walk_rows(2).to_count_sql_for(Dialect::Sqlite);
  let exists = walk_rows(2).to_exists_sql_for(Dialect::Sqlite);

  assert!(
    counted.0.starts_with(r#"WITH RECURSIVE "walk""#),
    "got: {}",
    counted.0
  );
  assert!(
    counted
      .0
      .contains(r#") SELECT COUNT(*) FROM (SELECT "walk"."id""#),
    "got: {}",
    counted.0
  );
  assert!(
    counted.0.ends_with(r#") AS "toolu_count""#),
    "got: {}",
    counted.0
  );
  assert!(
    exists.0.starts_with(r#"WITH RECURSIVE "walk""#),
    "got: {}",
    exists.0
  );
  assert!(
    exists.0.contains(r#") SELECT EXISTS(SELECT 1 FROM "walk""#),
    "got: {}",
    exists.0
  );
  assert_eq!(counted.1, vec![text("b1"), Value::Integer(2)]);
}

/// A CTE name is an ordinary quoted identifier, so `table_ref()` is what the
/// outer `FROM` names — and a hostile name doubles its quotes on both sides.
#[test]
fn a_hostile_cte_name_is_quote_doubled_in_the_prefix_and_in_the_from() {
  let cte = Cte::new(r#"x"; DROP TABLE edges; --"#, by_repo("r1"));
  let source = cte.table_ref();
  let (sql, _) = SelectBuilder::from_table(source)
    .columns_raw(&["id"])
    .with(cte)
    .to_sql_for(Dialect::Sqlite);

  assert!(
    sql.starts_with(r#"WITH "x""; DROP TABLE edges; --" AS ("#),
    "got: {sql}"
  );
  assert!(
    sql.ends_with(r#"FROM "x""; DROP TABLE edges; --""#),
    "got: {sql}"
  );
}
