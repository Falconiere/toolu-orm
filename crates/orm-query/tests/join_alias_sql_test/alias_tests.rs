//! Alias rendering, compound `ON` clauses, and qualified projections.

use super::fixtures::{C_ID, C_KIND, C_PATH, C_REPO, F_ID, F_LIVE, F_PATH, F_REPO};
use toolu_orm_core::alias::{QualifiedColumn, TableRef};
use toolu_orm_core::column::{Integer, Text};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::query_column::{Column, CommonOps};
use toolu_orm_core::value::Value;
use toolu_orm_query::select::SelectBuilder;

const USER_ID: Column<Text> = Column::new("users", "id");
const PIPELINE_USER_ID: Column<Text> = Column::new("pipelines", "user_id");

#[test]
fn aliased_from_and_join_render_table_as_alias() {
  let old = TableRef::aliased("memories", "old");
  let newer = TableRef::aliased("memories", "newer");
  let owner: Column<Text> = Column::new("memories", "owner");
  let created: Column<Integer> = Column::new("memories", "created_at");

  let (sql, params) = SelectBuilder::from_table(&old)
    .columns_qualified(&[&old.column(&owner)])
    .join(
      &newer,
      old
        .column(&owner)
        .equals(&newer.column(&owner))
        .and(old.column(&created).less_than(&newer.column(&created))),
    )
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    concat!(
      r#"SELECT "old"."owner" FROM "memories" AS "old" INNER JOIN "memories" AS "newer""#,
      r#" ON ("old"."owner" = "newer"."owner" AND "old"."created_at" < "newer"."created_at")"#
    )
  );
  assert!(params.is_empty());

  // A `TableRef` with no alias renders exactly what a `&str` table renders.
  let (plain, _) = SelectBuilder::from_table(TableRef::new("memories"))
    .columns_raw(&["id"])
    .to_sql_for(Dialect::Sqlite);
  let (as_str, _) = SelectBuilder::new("memories")
    .columns_raw(&["id"])
    .to_sql_for(Dialect::Sqlite);
  assert_eq!(plain, as_str);
  assert_eq!(plain, r#"SELECT "id" FROM "memories""#);
}

#[test]
fn str_table_join_renders_exactly_what_it_rendered_before() {
  let (inner, inner_params) = SelectBuilder::new("users")
    .columns_raw(&["id", "email"])
    .join("pipelines", USER_ID.equals(&PIPELINE_USER_ID))
    .to_sql_for(Dialect::Sqlite);
  assert_eq!(
    inner,
    concat!(
      r#"SELECT "id", "email" FROM "users" INNER JOIN "pipelines""#,
      r#" ON "users"."id" = "pipelines"."user_id""#
    )
  );
  assert!(inner_params.is_empty());

  let (left, left_params) = SelectBuilder::new("users")
    .columns_raw(&["id", "email"])
    .left_join("pipelines", USER_ID.equals(&PIPELINE_USER_ID))
    .to_sql_for(Dialect::Sqlite);
  assert_eq!(
    left,
    concat!(
      r#"SELECT "id", "email" FROM "users" LEFT JOIN "pipelines""#,
      r#" ON "users"."id" = "pipelines"."user_id""#
    )
  );
  assert!(left_params.is_empty());

  assert_eq!(SelectBuilder::new("users").table_name(), "users");
  assert_eq!(
    SelectBuilder::from_table(TableRef::aliased("users", "u")).table_name(),
    "users",
    "table_name is the relation, not the alias"
  );
}

#[test]
fn compound_on_clause_binds_its_value_and_parenthesizes() {
  let (sql, params) = SelectBuilder::new("code_symbols")
    .columns_raw(&["id"])
    .left_join(
      "code_feedback",
      F_REPO
        .equals(&C_REPO)
        .and(F_PATH.equals(&C_PATH))
        .and(F_LIVE.eq(1)),
    )
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    concat!(
      r#"SELECT "id" FROM "code_symbols" LEFT JOIN "code_feedback" ON "#,
      r#"(("code_feedback"."repo" = "code_symbols"."repo""#,
      r#" AND "code_feedback"."path" = "code_symbols"."path")"#,
      r#" AND "code_feedback"."live" = ?1)"#
    )
  );
  assert_eq!(params, vec![Value::Integer(1)]);
}

#[test]
fn on_or_clause_renders_or_with_parentheses() {
  let (sql, params) = SelectBuilder::new("code_symbols")
    .columns_raw(&["id"])
    .join("code_feedback", F_REPO.equals(&C_REPO).or(F_LIVE.eq(0)))
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    concat!(
      r#"SELECT "id" FROM "code_symbols" INNER JOIN "code_feedback" ON "#,
      r#"("code_feedback"."repo" = "code_symbols"."repo" OR "code_feedback"."live" = ?1)"#
    )
  );
  assert_eq!(params, vec![Value::Integer(0)]);
}

#[test]
fn qualified_projection_and_output_alias_render_qualified() {
  let c = TableRef::aliased("code_symbols", "c");
  let f = TableRef::aliased("code_feedback", "f");

  let (sql, params) = SelectBuilder::from_table(&c)
    .column_as(&c.column(&C_ID), "c_id")
    .column_as(&f.column(&F_ID), "f_id")
    .left_join(&f, c.column(&C_REPO).equals(&f.column(&F_REPO)))
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    concat!(
      r#"SELECT "c"."id" AS "c_id", "f"."id" AS "f_id" FROM "code_symbols" AS "c""#,
      r#" LEFT JOIN "code_feedback" AS "f" ON "c"."repo" = "f"."repo""#
    )
  );
  assert!(params.is_empty());

  // columns_qualified takes a plain column and an aliased one in one slice.
  // The unaliased table is named by its own name, so both qualifiers resolve
  // against the FROM/JOIN list this query actually declares.
  let f_live = f.column(&F_LIVE);
  let mixed: Vec<&dyn QualifiedColumn> = vec![&C_KIND, &f_live];
  let (sql, _) = SelectBuilder::from_table("code_symbols")
    .columns_qualified(&mixed)
    .left_join(&f, f.column(&F_REPO).equals(&C_REPO))
    .to_sql_for(Dialect::Sqlite);
  assert_eq!(
    sql,
    concat!(
      r#"SELECT "code_symbols"."kind", "f"."live" FROM "code_symbols""#,
      r#" LEFT JOIN "code_feedback" AS "f" ON "f"."repo" = "code_symbols"."repo""#
    )
  );

  // columns_typed keeps its unqualified rendering.
  let (bare, _) = SelectBuilder::new("code_symbols")
    .columns_typed(&[&C_ID, &C_KIND])
    .to_sql_for(Dialect::Sqlite);
  assert_eq!(bare, r#"SELECT "id", "kind" FROM "code_symbols""#);
}
