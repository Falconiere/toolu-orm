//! Every macro expands with `toolu-orm` as the only dependency.
//!
//! # Public API
//!
//! Tests: `#[table]` schema and companion column module, the generated
//! builders, a `#[view]` struct, `#[derive(ColumnEnum)]`, and
//! `#[derive(Relational)]` decoding a real JSON row.
//!
//! This file deliberately does **not** glob `toolu_orm::prelude`. This package
//! depends on `toolu-orm` alone, so `toolu_orm_core`, `toolu_orm_query`,
//! `serde` and `serde_json` are absent from the extern prelude — a `use` here
//! could not reach the companion column module's nested scope anyway. Every
//! path the expansions emit has to resolve on its own.

use toolu_orm::core::column::{EnumSchema, Integer, Text};
use toolu_orm::core::dialect::Dialect;
use toolu_orm::core::query_column::CommonOps;
use toolu_orm::core::relational_row::FromRelationalRow;
use toolu_orm::core::serde_json::{self, Value};
use toolu_orm::core::table::TableSchema;
use toolu_orm::{table, ColumnEnum, Relational};

#[table(name = "facade_only_users")]
#[view(FacadeOnlyUserPreview, pick(id, email))]
pub struct FacadeOnlyUser {
  #[column(primary_key)]
  pub id: Text,
  #[column(not_null)]
  pub email: Text,
  pub age: Integer,
}

#[derive(
  ColumnEnum, toolu_orm::core::serde::Serialize, toolu_orm::core::serde::Deserialize, Debug,
)]
#[serde(crate = "toolu_orm::core::serde", rename_all = "snake_case")]
pub enum FacadeOnlyStatus {
  Draft,
  InReview,
  Published,
}

#[derive(Debug, toolu_orm::core::serde::Deserialize)]
#[serde(crate = "toolu_orm::core::serde")]
pub struct FacadeOnlyPost {
  pub id: String,
  pub title: String,
}

#[derive(Relational)]
#[relational(table = "facade_only_users")]
pub struct FacadeOnlyUserWithPosts {
  pub id: String,
  pub email: String,
  #[has_many(
    table = "facade_only_posts",
    foreign_key = "author_id",
    columns = ["id", "title"]
  )]
  pub posts: Vec<FacadeOnlyPost>,
}

fn row(json: &str) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
  Ok(serde_json::from_str(json)?)
}

#[test]
fn table_macro_expands_without_the_prelude() {
  let def = FacadeOnlyUser::table_def();

  assert_eq!(def.name, "facade_only_users");
  assert_eq!(
    def
      .columns
      .iter()
      .map(|c| c.name.as_str())
      .collect::<Vec<_>>(),
    vec!["id", "email", "age"]
  );
  assert_eq!(facade_only_users::TABLE, "facade_only_users");
  assert_eq!(facade_only_users::ALL_COLUMNS, &["id", "email", "age"]);
}

#[test]
fn companion_column_module_holds_typed_columns() {
  // The nested `mod` is the case the prelude could never fix: these constants
  // are `Column<Text>` / `Column<Integer>` named through the resolved path.
  assert_eq!(
    facade_only_users::email.qualified(),
    r#""facade_only_users"."email""#
  );

  let (sql, params) = FacadeOnlyUser::select()
    .columns_typed(&[&facade_only_users::id, &facade_only_users::email])
    .order_by(facade_only_users::email.asc())
    .to_sql();

  assert_eq!(
    sql,
    r#"SELECT "id", "email" FROM "facade_only_users" ORDER BY "facade_only_users"."email" ASC"#
  );
  assert!(params.is_empty());

  // Pinned to one dialect: `to_sql_fragment` follows the active driver
  // feature, and this suite runs on the default and postgres lanes both.
  let (filter, filter_params) = facade_only_users::age
    .eq(30_i64)
    .to_sql_fragment_for(1, Dialect::Postgres);
  assert_eq!(filter, r#""facade_only_users"."age" = $1"#);
  assert_eq!(filter_params.len(), 1);
}

#[test]
fn generated_builders_reach_the_query_crate() {
  let (sql, params) = FacadeOnlyUser::select()
    .columns_raw(&["id", "email"])
    .to_sql();

  assert_eq!(sql, r#"SELECT "id", "email" FROM "facade_only_users""#);
  assert!(params.is_empty());
}

#[test]
fn view_struct_serializes_through_the_re_exported_serde() -> Result<(), Box<dyn std::error::Error>>
{
  let preview = FacadeOnlyUserPreview {
    id: "u1".to_owned(),
    email: "a@example.com".to_owned(),
  };

  assert_eq!(
    serde_json::to_string(&preview)?,
    r#"{"id":"u1","email":"a@example.com"}"#
  );
  Ok(())
}

#[test]
fn column_enum_derive_reports_renamed_variants() {
  assert_eq!(
    FacadeOnlyStatus::variants(),
    &["draft", "in_review", "published"]
  );
}

#[test]
fn relational_derive_decodes_a_json_row() -> Result<(), Box<dyn std::error::Error>> {
  let values = row(r#"["u1", "a@example.com", [["p1", "First"], ["p2", "Second"]]]"#)?;

  let user = FacadeOnlyUserWithPosts::from_relational_values(&values)?;

  assert_eq!(FacadeOnlyUserWithPosts::SCALAR_COLUMNS, &["id", "email"]);
  assert_eq!(user.id, "u1");
  assert_eq!(user.email, "a@example.com");
  assert_eq!(user.posts.len(), 2);
  assert_eq!(user.posts.first().ok_or("missing post")?.title, "First");
  Ok(())
}

#[test]
fn relational_derive_decodes_a_null_relation_as_empty() -> Result<(), Box<dyn std::error::Error>> {
  let values = row(r#"["u2", "b@example.com", null]"#)?;

  let user = FacadeOnlyUserWithPosts::from_relational_values(&values)?;

  assert_eq!(user.id, "u2");
  assert!(user.posts.is_empty());
  Ok(())
}
