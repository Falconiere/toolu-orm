//! SQL shape for relation projections that declare binary columns: SQLite gets
//! a NULL-preserving `hex()` wrapper, Postgres `encode(…, 'hex')`, and a
//! projection built from the plain string API keeps its previous SQL.

use toolu_orm_query::select::{RelationColumn, RelationalSelectBuilder};

fn owner_files_builder() -> RelationalSelectBuilder {
  RelationalSelectBuilder::new("owners", &["id"]).with_many_columns(
    "files",
    "files",
    "id",
    "owner_id",
    &[RelationColumn::new("id"), RelationColumn::binary("payload")],
  )
}

#[test]
fn sqlite_wraps_a_binary_column_in_a_null_preserving_hex_case() {
  let sql = owner_files_builder().to_sql_sqlite();
  assert!(
    sql
      .contains(r#"CASE WHEN "files"."payload" IS NULL THEN NULL ELSE hex("files"."payload") END"#),
    "binary column should be hex-encoded with a NULL guard, got: {sql}"
  );
  assert!(
    sql.contains(r#"json_array("files"."id", CASE WHEN"#),
    "non-binary columns keep their plain qualified form, got: {sql}"
  );
}

#[test]
fn sqlite_has_one_wraps_a_binary_column_too() {
  let sql = RelationalSelectBuilder::new("files", &["id"])
    .with_one_columns(
      "owner",
      "owners",
      "owner_id",
      "id",
      &[RelationColumn::new("id"), RelationColumn::binary("avatar")],
    )
    .to_sql_sqlite();
  assert!(
    sql
      .contains(r#"CASE WHEN "owners"."avatar" IS NULL THEN NULL ELSE hex("owners"."avatar") END"#),
    "has-one binary column should be hex-encoded, got: {sql}"
  );
  assert!(sql.contains("LIMIT 1"), "has-one keeps LIMIT 1, got: {sql}");
}

#[test]
fn postgres_encodes_a_binary_column_as_hex() {
  let sql = owner_files_builder().to_sql_postgres();
  assert!(
    sql.contains(r#"encode("files_sub"."payload", 'hex')"#),
    "binary column should use encode(…, 'hex'), got: {sql}"
  );
  assert!(
    sql.contains(r#"json_build_array("files_sub"."id", encode("#),
    "non-binary columns keep their plain qualified form, got: {sql}"
  );
}

#[test]
fn postgres_has_one_encodes_a_binary_column_too() {
  let sql = RelationalSelectBuilder::new("files", &["id"])
    .with_one_columns(
      "owner",
      "owners",
      "owner_id",
      "id",
      &[RelationColumn::new("id"), RelationColumn::binary("avatar")],
    )
    .to_sql_postgres();
  assert!(
    sql.contains(r#"encode("owner_sub"."avatar", 'hex')"#),
    "has-one binary column should use encode(…, 'hex'), got: {sql}"
  );
}

#[test]
fn a_projection_without_binary_columns_emits_no_encoder() {
  let string_api = RelationalSelectBuilder::new("owners", &["id"]).with_many(
    "files",
    "files",
    "id",
    "owner_id",
    &["id", "payload"],
  );
  let typed_api = RelationalSelectBuilder::new("owners", &["id"]).with_many_columns(
    "files",
    "files",
    "id",
    "owner_id",
    &[RelationColumn::new("id"), RelationColumn::new("payload")],
  );

  for sql in [string_api.to_sql_sqlite(), typed_api.to_sql_sqlite()] {
    assert!(
      !sql.contains("hex("),
      "undeclared columns stay raw, got: {sql}"
    );
    assert!(
      sql.contains(r#"json_array("files"."id", "files"."payload")"#),
      "undeclared columns stay raw, got: {sql}"
    );
  }
  for sql in [string_api.to_sql_postgres(), typed_api.to_sql_postgres()] {
    assert!(
      !sql.contains("encode("),
      "undeclared columns stay raw, got: {sql}"
    );
    assert!(
      sql.contains(r#"json_build_array("files_sub"."id", "files_sub"."payload")"#),
      "undeclared columns stay raw, got: {sql}"
    );
  }
}

#[test]
fn relation_columns_keep_their_declaration_in_the_config() -> Result<(), Box<dyn std::error::Error>>
{
  let builder = owner_files_builder();
  let rel = builder
    .relation_configs()
    .first()
    .ok_or("missing relation")?;
  let id = rel.target_columns.first().ok_or("missing id column")?;
  let payload = rel.target_columns.get(1).ok_or("missing payload column")?;
  assert_eq!(id.name(), "id");
  assert!(!id.is_binary());
  assert_eq!(payload.name(), "payload");
  assert!(payload.is_binary());
  assert_eq!(RelationColumn::from("id"), RelationColumn::new("id"));
  Ok(())
}
