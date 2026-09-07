# Relations

A relational load fetches a parent row and its children in **one statement**: the
children come back as a JSON array column, so there is no second round trip and
no N+1.

- **SQLite** renders correlated subqueries with `json_group_array`.
- **Postgres** renders `LEFT JOIN LATERAL` with `json_agg` / `json_build_array`.

## Declaring the shape

`#[derive(Relational)]` describes the result: scalar fields first, then one field
per relation. It implements `FromRelationalRow`, which builds the struct from the
decoded JSON values, and exposes `SCALAR_COLUMNS`.

```rust
use toolu_orm_macros::Relational;

#[derive(Relational)]
#[relational(table = "users")]
struct UserWithPosts {
  pub id: String,
  pub name: String,
  #[has_many(table = "posts", foreign_key = "author_id", columns = ["id", "title"])]
  pub posts: Vec<PostSummary>,
}

#[derive(Relational)]
#[relational(table = "posts")]
struct PostWithAuthor {
  pub id: String,
  pub title: String,
  #[belongs_to(table = "users", foreign_key = "author_id", columns = ["id", "name"])]
  pub author: Option<AuthorRow>,
}
```

| Attribute | Field type | Extra keys |
|---|---|---|
| `#[has_many(table, foreign_key, columns)]` | `Vec<T>` | — |
| `#[belongs_to(table, foreign_key, columns)]` | `Option<T>` | — |
| `#[many_to_many(table, foreign_key, columns, through, local_key)]` | `Vec<T>` | `through` join table, `local_key` on it |

`columns` is the target's column list **and** the JSON array order, so the
related struct's fields must line up with it. Related structs are decoded with
serde, so they derive `Deserialize`.

## Building the query

`RelationalQuery<T>` tracks the result tuple at compile time: each `with_many`
appends a `Vec<R>`, each `with_one` appends an `Option<R>`.

```rust
use toolu_orm_query::relational_builder::RelationalQuery;

let q = RelationalQuery::<(UserRow,)>::new("users", &["id", "name"])
  .with_many::<PostRow>("posts", "posts", "id", "author_id", &["id", "title"])
  .with_one::<ProfileRow>("profile", "profiles", "id", "user_id", &["id", "bio"]);

let _: RelationalQuery<(UserRow, Vec<PostRow>, Option<ProfileRow>)> = q;
```

Both methods take the same five arguments: the result field name, the target
table, the local key on the source table, the foreign key on the target, and the
target's column list.

`to_sql_sqlite()`, `to_sql_postgres()` and `to_sql()` render the statement. It
takes no parameters, so it runs directly on the driver.

## Reading the rows

Scalar columns come back first, in order, followed by one JSON column per
relation. Decode the JSON columns with `parse_many_column` / `parse_one_column`,
then hand the whole row to `from_relational_values`:

```rust
use toolu_orm_core::relational_row::FromRelationalRow;

let q = RelationalQuery::<(UserRow,)>::new("users", UserWithPosts::SCALAR_COLUMNS)
  .with_many::<PostRow>("posts", "posts", "id", "author_id", &["id", "title"]);

let mut rows = conn.query(&q.to_sql_sqlite(), ()).await?;
while let Some(row) = rows.next().await? {
  let id: String = row.get(0)?;
  let name: String = row.get(1)?;
  let posts_json: String = row.get(2)?;

  let posts = RelationalQuery::<(UserRow,)>::parse_many_column(&posts_json)?;
  let values = vec![
    serde_json::Value::String(id),
    serde_json::Value::String(name),
    serde_json::Value::Array(posts.into_iter().map(serde_json::Value::Array).collect()),
  ];
  let user = UserWithPosts::from_relational_values(&values)?;
}
```

Passing `UserWithPosts::SCALAR_COLUMNS` as the source column list is what keeps
the query and the struct in step.

A parent with no children decodes to an empty `Vec`, and a `belongs_to` with a
`NULL` foreign key decodes to `None` — both are covered by the
[relational-loads scenario](https://github.com/Falconiere/toolu-orm/blob/main/docs/scenarios/relational-loads.md).

## Untyped variant

`RelationalSelectBuilder` (in `toolu_orm_query::select`) has the same
`with_many` / `with_one` plus `to_sql_sqlite()` / `to_sql_postgres()` without the
type-state tuple, for when you only want the SQL.
