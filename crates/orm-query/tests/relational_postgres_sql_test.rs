use toolu_orm_query::select::RelationalSelectBuilder;

#[test]
fn postgres_single_has_many_generates_lateral_join() {
  let builder = RelationalSelectBuilder::new("users", &["id", "name"]).with_many(
    "posts",
    "posts",
    "id",
    "author_id",
    &["id", "title"],
  );
  let sql = builder.to_sql_postgres();
  assert!(
    sql.contains("LEFT JOIN LATERAL"),
    "should contain LEFT JOIN LATERAL, got: {sql}"
  );
  assert!(
    sql.contains("json_agg"),
    "should contain json_agg, got: {sql}"
  );
  assert!(
    sql.contains("json_build_array"),
    "should contain json_build_array, got: {sql}"
  );
  assert!(
    sql.contains(r#""posts_sub"."author_id" = "users"."id""#),
    "should contain join condition, got: {sql}"
  );
}

#[test]
fn postgres_has_one_generates_lateral_join_with_limit() {
  let builder = RelationalSelectBuilder::new("posts", &["id", "title"]).with_one(
    "author",
    "users",
    "author_id",
    "id",
    &["id", "name"],
  );
  let sql = builder.to_sql_postgres();
  assert!(
    sql.contains("LEFT JOIN LATERAL"),
    "should contain LEFT JOIN LATERAL, got: {sql}"
  );
  assert!(
    sql.contains("LIMIT 1"),
    "has-one should contain LIMIT 1, got: {sql}"
  );
}

#[test]
fn postgres_multiple_relations_generate_multiple_lateral_joins() {
  let builder = RelationalSelectBuilder::new("users", &["id", "name"])
    .with_many("posts", "posts", "id", "author_id", &["id", "title"])
    .with_many("comments", "comments", "id", "user_id", &["id", "body"]);
  let sql = builder.to_sql_postgres();
  let lateral_count = sql.matches("LEFT JOIN LATERAL").count();
  assert_eq!(
    lateral_count, 2,
    "should have 2 lateral joins, got {lateral_count} in: {sql}"
  );
}

#[test]
fn postgres_source_columns_are_qualified() {
  let builder = RelationalSelectBuilder::new("users", &["id", "name", "email"]).with_many(
    "posts",
    "posts",
    "id",
    "author_id",
    &["id", "title"],
  );
  let sql = builder.to_sql_postgres();
  assert!(
    sql.contains(r#""users"."id""#),
    "should qualify source columns, got: {sql}"
  );
  assert!(
    sql.contains(r#""users"."name""#),
    "should qualify source columns, got: {sql}"
  );
  assert!(
    sql.contains(r#""users"."email""#),
    "should qualify source columns, got: {sql}"
  );
}

#[test]
fn postgres_coalesce_wraps_json_agg() {
  let builder = RelationalSelectBuilder::new("users", &["id"]).with_many(
    "posts",
    "posts",
    "id",
    "author_id",
    &["id"],
  );
  let sql = builder.to_sql_postgres();
  assert!(
    sql.contains("coalesce"),
    "should wrap json_agg with coalesce, got: {sql}"
  );
  assert!(
    sql.contains("'[]'::json"),
    "coalesce fallback should be empty JSON array, got: {sql}"
  );
}

#[test]
fn postgres_on_true_terminates_lateral_join() {
  let builder = RelationalSelectBuilder::new("users", &["id"]).with_many(
    "posts",
    "posts",
    "id",
    "author_id",
    &["id"],
  );
  let sql = builder.to_sql_postgres();
  assert!(
    sql.contains("ON true"),
    "lateral join should end with ON true, got: {sql}"
  );
}
