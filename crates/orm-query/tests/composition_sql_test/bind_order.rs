//! One statement carrying every binding clause at once, pinned as an exact
//! `(sql, params)` pair per dialect.
//!
//! Bind numbering across a statement boundary is the part that corrupts data
//! silently when it is wrong: a value lands under the wrong placeholder and no
//! engine complains. So this asserts the whole rendering, both dialects, and
//! that the indices run `1..=params.len()` with no repeat and no gap.

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::{OrderBy, Scalar};
use toolu_orm_core::query_column::CommonOps;
use toolu_orm_core::value::Value;
use toolu_orm_query::select::{Cte, SelectBuilder};

use crate::fixtures::{by_repo, seeds_source, REPO, SEED_VALUE, SYMBOL_ID};

fn text(value: &str) -> Value {
  Value::Text(value.to_owned())
}

/// A CTE body, a binding projection, a binding `FROM` function, a binding
/// `JOIN … ON`, a `WHERE` subquery, a `UNION` arm, and the row window — in one
/// builder. The arm projects the same two output names and the ordering names
/// an output alias, so the statement is valid on both engines, not merely
/// renderable.
fn everything() -> SelectBuilder {
  let seeds = seeds_source();
  SelectBuilder::from_table(&seeds)
    .with(Cte::new("hits", by_repo("r1")))
    .column_scalar(Scalar::bind("tag"), "tag")
    .column_as(&seeds.column(&SEED_VALUE), "id")
    .join(
      "code_symbols",
      SYMBOL_ID
        .equals(&seeds.column(&SEED_VALUE))
        .and(SYMBOL_ID.ne("c9")),
    )
    .filter(Scalar::col(&SYMBOL_ID).in_subquery(by_repo("r2")))
    .union(
      SelectBuilder::new("code_symbols")
        .column_scalar(Scalar::sql("'x'"), "tag")
        .column_as(&SYMBOL_ID, "id")
        .filter(REPO.eq("r3")),
    )
    .order_by(OrderBy::alias_asc("id"))
    .limit(4)
    .offset(2)
}

fn expected_params() -> Vec<Value> {
  vec![
    text("r1"),
    text("tag"),
    text(r#"["a","b"]"#),
    text("c9"),
    text("r2"),
    text("r3"),
    Value::Integer(4),
    Value::Integer(2),
  ]
}

#[test]
fn every_binding_clause_numbers_in_render_order_on_sqlite() {
  let (sql, params) = everything().to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"WITH "hits" AS (SELECT "code_symbols"."id" FROM "code_symbols" "#.to_owned()
      + r#"WHERE "code_symbols"."repo" = ?1) "#
      + r#"SELECT ?2 AS "tag", "seeds"."value" AS "id" FROM json_each(?3) AS "seeds" "#
      + r#"INNER JOIN "code_symbols" ON ("code_symbols"."id" = "seeds"."value" "#
      + r#"AND "code_symbols"."id" != ?4) "#
      + r#"WHERE "code_symbols"."id" IN (SELECT "code_symbols"."id" FROM "code_symbols" "#
      + r#"WHERE "code_symbols"."repo" = ?5) "#
      + r#"UNION SELECT 'x' AS "tag", "code_symbols"."id" AS "id" FROM "code_symbols" "#
      + r#"WHERE "code_symbols"."repo" = ?6 "#
      + r#"ORDER BY "id" ASC LIMIT ?7 OFFSET ?8"#
  );
  assert_eq!(params, expected_params());
}

#[test]
fn the_same_statement_numbers_with_dollar_placeholders_on_postgres() {
  let (sql, params) = everything().to_sql_for(Dialect::Postgres);

  assert_eq!(
    sql,
    r#"WITH "hits" AS (SELECT "code_symbols"."id" FROM "code_symbols" "#.to_owned()
      + r#"WHERE "code_symbols"."repo" = $1) "#
      + r#"SELECT $2 AS "tag", "seeds"."value" AS "id" FROM json_each($3) AS "seeds" "#
      + r#"INNER JOIN "code_symbols" ON ("code_symbols"."id" = "seeds"."value" "#
      + r#"AND "code_symbols"."id" != $4) "#
      + r#"WHERE "code_symbols"."id" IN (SELECT "code_symbols"."id" FROM "code_symbols" "#
      + r#"WHERE "code_symbols"."repo" = $5) "#
      + r#"UNION SELECT 'x' AS "tag", "code_symbols"."id" AS "id" FROM "code_symbols" "#
      + r#"WHERE "code_symbols"."repo" = $6 "#
      + r#"ORDER BY "id" ASC LIMIT $7 OFFSET $8"#
  );
  assert_eq!(params, expected_params());
}

/// The structural invariant behind both assertions above, checked
/// programmatically so a future clause cannot quietly skip or repeat an index.
#[test]
fn the_placeholder_indices_run_from_one_with_no_gap_or_repeat() {
  for (dialect, marker) in [(Dialect::Sqlite, '?'), (Dialect::Postgres, '$')] {
    let (sql, params) = everything().to_sql_for(dialect);
    let indices: Vec<usize> = sql
      .match_indices(marker)
      .filter_map(|(at, _)| sql[at + 1..].split(|c: char| !c.is_ascii_digit()).next())
      .filter_map(|digits| digits.parse().ok())
      .collect();

    assert_eq!(
      indices,
      (1..=params.len()).collect::<Vec<usize>>(),
      "{dialect:?} placeholders out of order in: {sql}"
    );
  }
}

/// The count wrap re-numbers the nested statement from the start, because
/// nothing precedes it — but the `WITH` prefix still comes first.
#[test]
fn the_counted_form_of_the_same_statement_stays_self_consistent() {
  let (sql, params) = everything().to_count_sql_for(Dialect::Sqlite);

  assert!(sql.starts_with(r#"WITH "hits" AS ("#), "got: {sql}");
  assert!(sql.ends_with(r#") AS "toolu_count""#), "got: {sql}");
  assert!(!sql.contains(" ORDER BY "), "got: {sql}");
  assert!(!sql.contains(" LIMIT "), "got: {sql}");
  assert_eq!(
    params,
    vec![
      text("r1"),
      text("tag"),
      text(r#"["a","b"]"#),
      text("c9"),
      text("r2"),
      text("r3"),
    ]
  );
}
