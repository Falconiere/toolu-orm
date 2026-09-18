//! The copy issue #114 asks for: one statement, across `ATTACH`, carrying
//! NULLs, blobs and a projected default for a column the older store lacks.

use toolu_orm_core::alias::TableRef;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::Scalar;
use toolu_orm_query::insert::InsertBuilder;
use toolu_orm_query::select::SelectBuilder;

use super::db::{target_rows, TestResult, BLOB_BYTES, BLOB_OID, INDEXED_AT, PATH, RANK, REPO};
use super::support::{Attached, OLD};

/// `INSERT OR IGNORE INTO "main"."indexed_files" (…) SELECT … FROM "old"."indexed_files"`
/// with `rank` projected as a bound default, because the older store has no
/// such column.
fn copy_statement() -> InsertBuilder {
  let older = SelectBuilder::from_table(TableRef::new("indexed_files").in_database(OLD))
    .columns_raw(&["repo", "path", "blob_oid", "indexed_at"])
    .column_scalar(Scalar::bind(7_i64), "rank");

  InsertBuilder::into_table(TableRef::new("indexed_files").in_database("main"))
    .or_ignore()
    .select(&[&REPO, &PATH, &BLOB_OID, &INDEXED_AT, &RANK], older)
}

#[test]
fn one_statement_copies_every_row_across_the_attachment() -> TestResult {
  let fixture = Attached::open("copy")?;
  let attached = fixture.attach()?;
  assert_eq!(attached.schema(), OLD);

  // The only call is execute(): it returns a count, never rows. No fetch runs,
  // so no source row is ever decoded into Rust on the way across.
  let affected = copy_statement().execute(&fixture.conn)?;
  assert_eq!(affected, 3, "every source row landed in one statement");

  let rows = target_rows(&fixture.conn)?;
  assert_eq!(rows.len(), 3);
  Ok(())
}

#[test]
fn a_null_text_and_a_null_blob_survive_the_copy_as_nulls() -> TestResult {
  let fixture = Attached::open("nulls")?;
  let attached = fixture.attach()?;
  copy_statement().execute(&fixture.conn)?;
  drop(attached);

  let rows = target_rows(&fixture.conn)?;
  let Some(nulled) = rows.iter().find(|r| r.path == "src/b.rs") else {
    return Err("the row with NULL columns was not copied".into());
  };
  assert_eq!(nulled.blob_oid, None, "a NULL blob stays NULL, not empty");
  assert_eq!(nulled.indexed_at, None, "a NULL integer stays NULL, not 0");
  Ok(())
}

#[test]
fn a_blob_round_trips_byte_identical_including_an_empty_one() -> TestResult {
  let fixture = Attached::open("blobs")?;
  let attached = fixture.attach()?;
  copy_statement().execute(&fixture.conn)?;
  drop(attached);

  let rows = target_rows(&fixture.conn)?;
  let Some(carried) = rows.iter().find(|r| r.path == "src/a.rs") else {
    return Err("the row carrying the blob was not copied".into());
  };
  assert_eq!(
    carried.blob_oid.as_deref(),
    Some(BLOB_BYTES),
    "the bytes never went through a String"
  );

  let Some(empty) = rows.iter().find(|r| r.path == "src/c.rs") else {
    return Err("the row carrying the empty blob was not copied".into());
  };
  assert_eq!(
    empty.blob_oid.as_deref(),
    Some(&[][..]),
    "an empty blob is a value, distinct from NULL"
  );
  Ok(())
}

#[test]
fn a_column_the_source_lacks_takes_the_projected_default() -> TestResult {
  let fixture = Attached::open("defaults")?;
  let attached = fixture.attach()?;
  copy_statement().execute(&fixture.conn)?;
  drop(attached);

  let rows = target_rows(&fixture.conn)?;
  assert!(
    rows.iter().all(|r| r.rank == 7),
    "the SELECT list supplied rank database-side, not the table DEFAULT of -1: {rows:?}"
  );
  Ok(())
}

#[test]
fn the_executed_statement_is_a_single_insert_select() {
  let (sql, params) = copy_statement().to_sql_for(Dialect::Sqlite);
  assert_eq!(
    sql,
    r#"INSERT OR IGNORE INTO "main"."indexed_files" ("repo", "path", "blob_oid", "indexed_at", "rank") SELECT "repo", "path", "blob_oid", "indexed_at", ?1 AS "rank" FROM "old"."indexed_files""#
  );
  assert_eq!(params.len(), 1, "only the projected default binds");
  assert!(!sql.contains(';'), "one statement, not a batch");
}

#[test]
fn a_source_with_no_rows_reports_zero_and_leaves_the_target_untouched() -> TestResult {
  let fixture = Attached::open("empty")?;
  let attached = fixture.attach()?;
  fixture.conn.execute("DELETE FROM old.indexed_files", [])?;

  let affected = copy_statement().execute(&fixture.conn)?;
  assert_eq!(affected, 0);
  drop(attached);

  assert!(target_rows(&fixture.conn)?.is_empty());
  Ok(())
}
