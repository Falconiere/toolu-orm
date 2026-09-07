use toolu_orm_core::relation::{many, one, RelationKind, RelationRegistry};

#[test]
fn new_registry_is_empty() {
  let registry = RelationRegistry::new();
  assert!(registry.get("users").is_none());
}

#[test]
fn register_and_get_returns_relations_for_table() -> Result<(), Box<dyn std::error::Error>> {
  let mut registry = RelationRegistry::new();
  registry.register(
    "users",
    vec![many("posts", "posts", &[("id", "author_id")])],
  );
  let rels = registry.get("users").ok_or("should find users relations")?;
  assert_eq!(rels.len(), 1);
  let r0 = rels.first().ok_or("missing r0")?;
  assert_eq!(r0.field_name, "posts");
  assert_eq!(r0.kind, RelationKind::Many);
  Ok(())
}

#[test]
fn register_multiple_tables() -> Result<(), Box<dyn std::error::Error>> {
  let mut registry = RelationRegistry::new();
  registry.register(
    "users",
    vec![many("posts", "posts", &[("id", "author_id")])],
  );
  registry.register(
    "posts",
    vec![one("author", "users", &[("author_id", "id")])],
  );
  assert_eq!(registry.get("users").ok_or("missing users")?.len(), 1);
  assert_eq!(registry.get("posts").ok_or("missing posts")?.len(), 1);
  assert!(registry.get("comments").is_none());
  Ok(())
}

#[test]
fn register_overwrites_existing_relations() -> Result<(), Box<dyn std::error::Error>> {
  let mut registry = RelationRegistry::new();
  registry.register(
    "users",
    vec![many("posts", "posts", &[("id", "author_id")])],
  );
  registry.register(
    "users",
    vec![
      many("posts", "posts", &[("id", "author_id")]),
      many("comments", "comments", &[("id", "user_id")]),
    ],
  );
  let rels = registry.get("users").ok_or("missing users")?;
  assert_eq!(rels.len(), 2);
  Ok(())
}

#[test]
fn get_returns_none_for_unregistered_table() {
  let registry = RelationRegistry::new();
  assert!(registry.get("nonexistent").is_none());
}
