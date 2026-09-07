use toolu_orm_core::relational_row::{parse_json_array_of_arrays, RelationDeserializer};
use toolu_orm_query::relational_builder::RelationalQuery;

#[derive(Debug, Clone, PartialEq)]
struct UserRow {
  pub id: String,
  pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
struct PostRow {
  pub id: String,
  pub title: String,
}

#[test]
fn to_sql_postgres_generates_valid_sql() {
  let q = RelationalQuery::<(UserRow,)>::new("users", &["id", "name"]).with_many::<PostRow>(
    "posts",
    "posts",
    "id",
    "author_id",
    &["id", "title"],
  );
  let sql = q.to_sql_postgres();
  assert!(sql.contains("SELECT"), "should start with SELECT: {sql}");
  assert!(
    sql.contains("LEFT JOIN LATERAL"),
    "should contain lateral join: {sql}"
  );
  assert!(
    sql.contains(r#"FROM "users""#),
    "should contain FROM: {sql}"
  );
}

#[test]
fn to_sql_sqlite_generates_valid_sql() {
  let q = RelationalQuery::<(UserRow,)>::new("users", &["id", "name"]).with_many::<PostRow>(
    "posts",
    "posts",
    "id",
    "author_id",
    &["id", "title"],
  );
  let sql = q.to_sql_sqlite();
  assert!(sql.contains("SELECT"), "should start with SELECT: {sql}");
  assert!(
    sql.contains("json_group_array"),
    "should contain json_group_array: {sql}"
  );
  assert!(
    sql.contains(r#"FROM "users""#),
    "should contain FROM: {sql}"
  );
}

#[test]
fn deserialize_many_relation_from_json() -> Result<(), Box<dyn std::error::Error>> {
  let json = r#"[["post-1","Hello"],["post-2","World"]]"#;
  let rows = parse_json_array_of_arrays(json)?;
  assert_eq!(rows.len(), 2);

  let first_inner = rows.first().ok_or("missing first_inner")?;
  let second_inner = rows.get(1).ok_or("missing second_inner")?;
  let deser0 = RelationDeserializer::new(first_inner);
  let id: String = deser0.get(0)?;
  let title: String = deser0.get(1)?;
  assert_eq!(id, "post-1");
  assert_eq!(title, "Hello");

  let deser1 = RelationDeserializer::new(second_inner);
  let id: String = deser1.get(0)?;
  let title: String = deser1.get(1)?;
  assert_eq!(id, "post-2");
  assert_eq!(title, "World");
  Ok(())
}

#[test]
fn deserialize_empty_many_relation_from_json() -> Result<(), Box<dyn std::error::Error>> {
  let json = "[]";
  let rows = parse_json_array_of_arrays(json)?;
  assert_eq!(rows.len(), 0);
  Ok(())
}
