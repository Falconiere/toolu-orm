//! The copy across a Postgres schema qualifier, and the conflict modes on it.

use toolu_orm_core::alias::TableRef;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::Scalar;
use toolu_orm_query::insert::{InsertBuilder, OnConflict};
use toolu_orm_query::select::SelectBuilder;

use super::db;
use super::schema::{
  FileRow, TestResult, BLOB_BYTES, BLOB_OID, INDEXED_AT, PATH, RANK, REPO, SOURCE_COLUMNS,
};

fn target(schema: &str) -> TableRef {
  TableRef::new("file_index").in_database(schema)
}

/// `legacy_index` of the same schema, with `rank` projected as a literal.
fn older(schema: &str, rank: i64) -> SelectBuilder {
  SelectBuilder::from_table(TableRef::new("legacy_index").in_database(schema))
    .columns_raw(SOURCE_COLUMNS)
    .column_scalar(Scalar::bind(rank), "rank")
}

fn copy_into(builder: InsertBuilder, schema: &str, rank: i64) -> InsertBuilder {
  builder.select(
    &[&REPO, &PATH, &BLOB_OID, &INDEXED_AT, &RANK],
    older(schema, rank),
  )
}

/// Every landed row, ordered by key, read through the same schema qualifier.
async fn landed(
  client: &tokio_postgres::Client,
  schema: &str,
) -> Result<Vec<FileRow>, toolu_orm_query::QueryError> {
  SelectBuilder::from_table(target(schema))
    .columns_raw(&["repo", "path", "blob_oid", "indexed_at", "rank"])
    .order_by(REPO.asc())
    .order_by(PATH.asc())
    .fetch_all(client)
    .await
}

fn row_at<'a>(rows: &'a [FileRow], path: &str) -> Result<&'a FileRow, Box<dyn std::error::Error>> {
  rows
    .iter()
    .find(|row| row.path == path)
    .ok_or_else(|| format!("no row at {path}; landed: {rows:?}").into())
}

/// Pre-seed the target with the key the source also has.
async fn seed_conflicting_row(
  client: &tokio_postgres::Client,
  schema: &str,
) -> Result<(), Box<dyn std::error::Error>> {
  InsertBuilder::into_table(target(schema))
    .set(&REPO, "r1")
    .set(&PATH, "src/a.rs")
    .set(&INDEXED_AT, 999_i64)
    .set(&RANK, 42_i64)
    .execute(client)
    .await?;
  Ok(())
}

#[tokio::test]
async fn one_statement_copies_every_row_across_the_schema_qualifier() -> TestResult {
  let schema = "insert_select_copy";
  let client = db::setup_db(schema).await?;

  let affected = copy_into(InsertBuilder::into_table(target(schema)), schema, 7)
    .execute(&client)
    .await?;
  assert_eq!(affected, 3);

  let rows = landed(&client, schema).await?;
  assert_eq!(rows.len(), 3);
  assert_eq!(
    row_at(&rows, "src/a.rs")?.blob_oid.as_deref(),
    Some(BLOB_BYTES),
    "bytea round-trips byte-identical"
  );
  let nulled = row_at(&rows, "src/b.rs")?;
  assert_eq!(nulled.blob_oid, None, "a NULL bytea stays NULL");
  assert_eq!(nulled.indexed_at, None);
  assert_eq!(
    row_at(&rows, "src/c.rs")?.blob_oid.as_deref(),
    Some(&[][..]),
    "an empty bytea is a value, distinct from NULL"
  );
  assert!(
    rows.iter().all(|row| row.rank == 7),
    "the projected default, not the column DEFAULT of -1: {rows:?}"
  );
  Ok(())
}

#[tokio::test]
async fn or_ignore_becomes_on_conflict_do_nothing_and_keeps_the_stored_row() -> TestResult {
  let schema = "insert_select_ignore";
  let client = db::setup_db(schema).await?;
  seed_conflicting_row(&client, schema).await?;

  let affected = copy_into(
    InsertBuilder::into_table(target(schema)).or_ignore(),
    schema,
    7,
  )
  .execute(&client)
  .await?;
  assert_eq!(affected, 2, "one key was already present");

  let rows = landed(&client, schema).await?;
  let kept = row_at(&rows, "src/a.rs")?;
  assert_eq!(kept.indexed_at, Some(999), "the stored row survived");
  assert_eq!(kept.rank, 42);
  assert_eq!(rows.len(), 3);
  Ok(())
}

#[tokio::test]
async fn the_explicit_conflict_clause_runs_unwrapped_on_postgres() -> TestResult {
  let schema = "insert_select_upsert";
  let client = db::setup_db(schema).await?;
  seed_conflicting_row(&client, schema).await?;

  let upsert = copy_into(InsertBuilder::into_table(target(schema)), schema, 7).on_conflict(
    OnConflict::column(&REPO)
      .and_column(&PATH)
      .set_excluded(&INDEXED_AT)
      .set(&RANK, 5_i64),
  );

  // Postgres has no parse ambiguity here, so it gets the plain statement while
  // SQLite gets a derived-table guard. Both halves are asserted so a renderer
  // that starts wrapping Postgres fails here rather than silently.
  let (sql, _) = upsert.to_sql_for(Dialect::Postgres);
  assert!(
    !sql.contains("toolu_insert_source"),
    "the SQLite-only guard leaked into the Postgres rendering: {sql}"
  );

  let affected = upsert.execute(&client).await?;
  assert_eq!(affected, 3, "two inserts and one update");

  let rows = landed(&client, schema).await?;
  let updated = row_at(&rows, "src/a.rs")?;
  assert_eq!(
    updated.indexed_at,
    Some(100),
    "the proposed value replaced 999"
  );
  assert_eq!(updated.rank, 5);
  assert_eq!(rows.len(), 3);
  Ok(())
}

#[tokio::test]
async fn a_binding_source_and_a_binding_conflict_clause_keep_their_order() -> TestResult {
  let schema = "insert_select_binds";
  let client = db::setup_db(schema).await?;
  seed_conflicting_row(&client, schema).await?;

  // rank binds $1 in the source's projection; the conflict clause binds $2.
  // If the two were transposed the copy would land 11 as the rank and 22 as
  // the update, which is exactly the silent corruption this pins.
  let affected = copy_into(InsertBuilder::into_table(target(schema)), schema, 11)
    .on_conflict(
      OnConflict::column(&REPO)
        .and_column(&PATH)
        .set(&RANK, 22_i64),
    )
    .execute(&client)
    .await?;
  assert_eq!(affected, 3);

  let rows = landed(&client, schema).await?;
  assert_eq!(
    row_at(&rows, "src/a.rs")?.rank,
    22,
    "the conflicting row took the clause's value"
  );
  assert_eq!(
    row_at(&rows, "src/b.rs")?.rank,
    11,
    "an inserted row took the projection's"
  );
  Ok(())
}
