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

#[derive(Debug, Clone, PartialEq)]
struct CommentRow {
  pub id: String,
  pub body: String,
}

#[derive(Debug, Clone, PartialEq)]
struct ProfileRow {
  pub id: String,
  pub bio: String,
}

type UserWithPostsCommentsProfile =
  RelationalQuery<(UserRow, Vec<PostRow>, Vec<CommentRow>, Option<ProfileRow>)>;

#[test]
fn query_starts_with_base_type() {
  let _q: RelationalQuery<(UserRow,)> =
    RelationalQuery::<(UserRow,)>::new("users", &["id", "name"]);
}

#[test]
fn with_many_appends_vec_to_type() {
  let q = RelationalQuery::<(UserRow,)>::new("users", &["id", "name"]).with_many::<PostRow>(
    "posts",
    "posts",
    "id",
    "author_id",
    &["id", "title"],
  );
  let _: RelationalQuery<(UserRow, Vec<PostRow>)> = q;
}

#[test]
fn with_one_appends_option_to_type() {
  let q = RelationalQuery::<(PostRow,)>::new("posts", &["id", "title"]).with_one::<UserRow>(
    "author",
    "users",
    "author_id",
    "id",
    &["id", "name"],
  );
  let _: RelationalQuery<(PostRow, Option<UserRow>)> = q;
}

#[test]
fn chaining_multiple_relations() {
  let q = RelationalQuery::<(UserRow,)>::new("users", &["id", "name"])
    .with_many::<PostRow>("posts", "posts", "id", "author_id", &["id", "title"])
    .with_many::<CommentRow>("comments", "comments", "id", "user_id", &["id", "body"])
    .with_one::<ProfileRow>("profile", "profiles", "id", "user_id", &["id", "bio"]);
  let _: UserWithPostsCommentsProfile = q;
}

#[test]
fn builder_stores_relation_configs() -> Result<(), Box<dyn std::error::Error>> {
  let q = RelationalQuery::<(UserRow,)>::new("users", &["id", "name"]).with_many::<PostRow>(
    "posts",
    "posts",
    "id",
    "author_id",
    &["id", "title"],
  );
  let inner = q.inner_builder();
  assert_eq!(inner.relation_configs().len(), 1);
  assert_eq!(
    inner
      .relation_configs()
      .first()
      .ok_or("missing rel0")?
      .field_name,
    "posts"
  );
  Ok(())
}
