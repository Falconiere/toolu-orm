use toolu_orm_query::select::RelationalSelectBuilder;

#[test]
fn new_builder_has_no_relations() {
  let builder = RelationalSelectBuilder::new("users", &["id", "name", "email"]);
  let config = builder.relation_configs();
  assert_eq!(config.len(), 0);
}

#[test]
fn with_many_adds_relation_config() -> Result<(), Box<dyn std::error::Error>> {
  let builder = RelationalSelectBuilder::new("users", &["id", "name"]).with_many(
    "posts",
    "posts",
    "id",
    "author_id",
    &["id", "title", "body"],
  );
  let config = builder.relation_configs();
  assert_eq!(config.len(), 1);
  let c0 = config.first().ok_or("missing c0")?;
  assert_eq!(c0.field_name, "posts");
  assert_eq!(c0.target_table, "posts");
  assert_eq!(c0.local_key, "id");
  assert_eq!(c0.foreign_key, "author_id");
  assert!(c0.is_many);
  Ok(())
}

#[test]
fn with_one_adds_relation_config() -> Result<(), Box<dyn std::error::Error>> {
  let builder = RelationalSelectBuilder::new("posts", &["id", "title", "author_id"]).with_one(
    "author",
    "users",
    "author_id",
    "id",
    &["id", "name"],
  );
  let config = builder.relation_configs();
  assert_eq!(config.len(), 1);
  let c0 = config.first().ok_or("missing c0")?;
  assert_eq!(c0.field_name, "author");
  assert_eq!(c0.local_key, "author_id");
  assert_eq!(c0.foreign_key, "id");
  assert!(!c0.is_many);
  Ok(())
}

#[test]
fn chaining_multiple_relations() -> Result<(), Box<dyn std::error::Error>> {
  let builder = RelationalSelectBuilder::new("users", &["id", "name"])
    .with_many("posts", "posts", "id", "author_id", &["id", "title"])
    .with_many("comments", "comments", "id", "user_id", &["id", "body"])
    .with_one("profile", "profiles", "id", "user_id", &["id", "bio"]);
  let config = builder.relation_configs();
  assert_eq!(config.len(), 3);
  assert_eq!(config.first().ok_or("missing c0")?.field_name, "posts");
  assert_eq!(config.get(1).ok_or("missing c1")?.field_name, "comments");
  assert_eq!(config.get(2).ok_or("missing c2")?.field_name, "profile");
  Ok(())
}

#[test]
fn source_table_and_columns_are_stored() {
  let builder = RelationalSelectBuilder::new("users", &["id", "name", "email"]);
  assert_eq!(builder.source_table(), "users");
  assert_eq!(builder.source_columns(), &["id", "name", "email"]);
}
