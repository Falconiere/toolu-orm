use toolu_orm_query::select::RelationalSelectBuilder;

#[test]
fn sqlite_single_has_many_generates_correlated_subquery() {
  let builder = RelationalSelectBuilder::new("users", &["id", "name"]).with_many(
    "posts",
    "posts",
    "id",
    "author_id",
    &["id", "title"],
  );
  let sql = builder.to_sql_sqlite();
  assert!(
    sql.contains("json_group_array"),
    "should contain json_group_array, got: {sql}"
  );
  assert!(
    sql.contains("json_array"),
    "should contain json_array, got: {sql}"
  );
  assert!(
    sql.contains(r#""posts"."author_id" = "users"."id""#),
    "should contain join condition, got: {sql}"
  );
  assert!(
    !sql.contains("LATERAL"),
    "SQLite should not use LATERAL, got: {sql}"
  );
}

#[test]
fn sqlite_has_one_generates_subquery_without_aggregation() {
  let builder = RelationalSelectBuilder::new("posts", &["id", "title"]).with_one(
    "author",
    "users",
    "author_id",
    "id",
    &["id", "name"],
  );
  let sql = builder.to_sql_sqlite();
  assert!(
    sql.contains("json_array"),
    "should contain json_array, got: {sql}"
  );
  assert!(
    sql.contains("LIMIT 1"),
    "has-one should contain LIMIT 1, got: {sql}"
  );
}

#[test]
fn sqlite_multiple_relations_generate_multiple_subqueries() {
  let builder = RelationalSelectBuilder::new("users", &["id", "name"])
    .with_many("posts", "posts", "id", "author_id", &["id", "title"])
    .with_many("comments", "comments", "id", "user_id", &["id", "body"]);
  let sql = builder.to_sql_sqlite();
  let subquery_count = sql.matches("json_group_array").count();
  assert_eq!(
    subquery_count, 2,
    "should have 2 subqueries with json_group_array, got {subquery_count} in: {sql}"
  );
}

#[test]
fn sqlite_relation_column_is_aliased() {
  let builder = RelationalSelectBuilder::new("users", &["id"]).with_many(
    "posts",
    "posts",
    "id",
    "author_id",
    &["id", "title"],
  );
  let sql = builder.to_sql_sqlite();
  assert!(
    sql.contains(r#"AS "posts""#),
    "subquery should be aliased as the relation field name, got: {sql}"
  );
}

#[test]
fn sqlite_coalesce_wraps_json_group_array() {
  let builder = RelationalSelectBuilder::new("users", &["id"]).with_many(
    "posts",
    "posts",
    "id",
    "author_id",
    &["id"],
  );
  let sql = builder.to_sql_sqlite();
  assert!(
    sql.contains("coalesce"),
    "should wrap json_group_array with coalesce, got: {sql}"
  );
  assert!(
    sql.contains("json_array()"),
    "coalesce fallback should be empty json_array(), got: {sql}"
  );
}

#[test]
fn sqlite_source_columns_are_qualified() {
  let builder = RelationalSelectBuilder::new("users", &["id", "name"]).with_many(
    "posts",
    "posts",
    "id",
    "author_id",
    &["id"],
  );
  let sql = builder.to_sql_sqlite();
  assert!(
    sql.contains(r#""users"."id""#),
    "should qualify source columns, got: {sql}"
  );
  assert!(
    sql.contains(r#""users"."name""#),
    "should qualify source columns, got: {sql}"
  );
}
