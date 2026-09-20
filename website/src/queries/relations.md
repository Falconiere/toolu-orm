# Relations

A relational load fetches a parent row and its children in **one statement**: the
children come back as a JSON array column, so there is no second round trip and
no N+1.

- **SQLite** renders correlated subqueries with `json_group_array`.
- **Postgres** renders `LEFT JOIN LATERAL` with `json_agg` / `json_build_array`.

## Declaring the shape

`#[derive(Relational)]` describes the result. It implements `FromRelationalRow`,
which reads scalar values first, then one JSON value per relation, and exposes
`SCALAR_COLUMNS`. The derive handles decoding; you still configure the SQL
builder's joins and columns explicitly.

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

The attribute's `columns` list maps JSON array positions to field names. It must
match the builder's target column order. Related structs derive `Deserialize`;
their field declaration order is irrelevant because decoding uses those names.
Here `PostSummary` has `id: String` and `title: String`, and `AuthorRow` has
`id: String` and `name: String`; both derive `Deserialize`.
`many_to_many` supports decoding a supplied array of related rows, but the query
builders do not generate a join through its `through` table automatically.

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

For a belongs-to relation, put the source table's foreign key in the local-key slot
and the referenced key in the target-key slot. For example, posts with authors
use `.with_one::<AuthorRow>("author", "users", "author_id", "id", &["id", "name"])`.

`to_sql_sqlite()`, `to_sql_postgres()` and `to_sql()` render the statement. It
takes no parameters, so it runs directly on the driver. These relational builders
have no `fetch_*`, filtering, ordering or pagination methods; the tuple tracks
the declared shape rather than executing or decoding rows automatically.

## Reading the rows

Scalar columns come back first, in order, followed by one JSON column per
relation. Decode each SQLite JSON column with `decode_relation_json`, then hand
the whole row to `from_relational_values`. This example uses a raw libsql
connection:

```rust
use toolu_orm_core::{relational_row::FromRelationalRow, serde_json};

let q = RelationalQuery::<(UserRow,)>::new("users", UserWithPosts::SCALAR_COLUMNS)
  .with_many::<PostSummary>("posts", "posts", "id", "author_id", &["id", "title"]);

let mut rows = conn.query(&q.to_sql_sqlite(), ()).await?;
while let Some(row) = rows.next().await? {
  let id: String = row.get(0)?;
  let name: String = row.get(1)?;
  let posts_json: String = row.get(2)?;

  let posts = q.decode_relation_json("posts", &posts_json)?;
  let values = vec![
    serde_json::Value::String(id),
    serde_json::Value::String(name),
    posts,
  ];
  let user = UserWithPosts::from_relational_values(&values)?;
}
```

Passing `UserWithPosts::SCALAR_COLUMNS` as the source column list is what keeps
the query and the struct in step.

Postgres returns a JSON value instead of text; pass it to
`decode_relation_value(field, &value)`. For a missing to-one relation, the driver
column is SQL NULL: read it as an `Option` and supply `serde_json::Value::Null`.
The lower-level `parse_many_column` and `parse_one_column` helpers parse JSON
arrays only; they do not decode binary columns.

A parent with no children decodes to an empty `Vec`, and a `belongs_to` with a
`NULL` foreign key decodes to `None` — both are covered by the
[relational-loads scenario](https://github.com/Falconiere/toolu-orm/blob/main/docs/scenarios/relational-loads.md).

## Binary relation columns

JSON cannot carry database binary values directly. Use `with_many_columns` or
`with_one_columns` and declare each BLOB / bytea column explicitly:

```rust
use toolu_orm_query::select::RelationColumn;

let q = RelationalQuery::<(UserRow,)>::new("users", &["id", "name"])
  .with_many_columns::<FileRow>("files", "files", "id", "owner_id", &[
    RelationColumn::new("id"),
    RelationColumn::binary("payload"),
  ]);
```

The SQL transports binary data as hex. `decode_relation_json` /
`decode_relation_value` restore byte arrays, preserving NULL and empty blobs
separately, so related `Vec<u8>` and `Option<Vec<u8>>` fields deserialize normally.

## Untyped variant

`RelationalSelectBuilder` (in `toolu_orm_query::select`) has the same
`with_many` / `with_one`, binary-column variants and decoding helpers, plus
`to_sql_sqlite()` / `to_sql_postgres()` without the type-state tuple.
