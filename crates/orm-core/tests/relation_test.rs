use toolu_orm_core::relation::{many, many_through, one, JoinColumn, RelationKind, ThroughDef};

#[test]
fn one_creates_relation_with_single_join_column() {
  let rel = one("author", "users", &[("author_id", "id")]);
  assert_eq!(rel.field_name, "author");
  assert_eq!(rel.target_table, "users");
  assert_eq!(rel.kind, RelationKind::One);
  assert_eq!(rel.join_columns.len(), 1);
  assert_eq!(
    rel.join_columns.first(),
    Some(&JoinColumn {
      local: "author_id".to_owned(),
      foreign: "id".to_owned(),
    })
  );
  assert!(rel.through.is_none());
}

#[test]
fn many_creates_relation_with_single_join_column() {
  let rel = many("posts", "posts", &[("id", "author_id")]);
  assert_eq!(rel.field_name, "posts");
  assert_eq!(rel.target_table, "posts");
  assert_eq!(rel.kind, RelationKind::Many);
  assert_eq!(rel.join_columns.len(), 1);
  assert_eq!(
    rel.join_columns.first(),
    Some(&JoinColumn {
      local: "id".to_owned(),
      foreign: "author_id".to_owned(),
    })
  );
  assert!(rel.through.is_none());
}

#[test]
fn many_supports_composite_join_columns() -> Result<(), Box<dyn std::error::Error>> {
  let rel = many(
    "assignments",
    "assignments",
    &[("org_id", "org_id"), ("user_id", "user_id")],
  );
  assert_eq!(rel.join_columns.len(), 2);
  let c0 = rel.join_columns.first().ok_or("missing c0")?;
  let c1 = rel.join_columns.get(1).ok_or("missing c1")?;
  assert_eq!(c0.local, "org_id");
  assert_eq!(c0.foreign, "org_id");
  assert_eq!(c1.local, "user_id");
  assert_eq!(c1.foreign, "user_id");
  Ok(())
}

#[test]
fn many_through_creates_junction_relation() {
  let rel = many_through(
    "tags",
    "tags",
    &[("id", "id")],
    "post_tags",
    "post_id",
    "tag_id",
  );
  assert_eq!(rel.field_name, "tags");
  assert_eq!(rel.target_table, "tags");
  assert_eq!(rel.kind, RelationKind::Many);
  assert_eq!(
    rel.through,
    Some(ThroughDef {
      junction_table: "post_tags".to_owned(),
      local_column: "post_id".to_owned(),
      foreign_column: "tag_id".to_owned(),
    })
  );
}

#[test]
fn relation_kind_debug_format() {
  assert_eq!(format!("{:?}", RelationKind::One), "One");
  assert_eq!(format!("{:?}", RelationKind::Many), "Many");
}

#[test]
fn join_column_clone_produces_equal_copy() {
  let col = JoinColumn {
    local: "a".to_owned(),
    foreign: "b".to_owned(),
  };
  let cloned = col.clone();
  assert_eq!(col, cloned);
}
