//! Relational loads over a real `BLOB` column on a real in-memory libsql
//! database: the same declaration, SQL and decoding as the rusqlite suite, which
//! is also what proves libsql accepts `hex()` inside `json_array`.

#[path = "fixtures/libsql_blob_db.rs"]
pub mod db;

use serde::Deserialize;
use toolu_orm_core::relational_row::FromRelationalRow;
use toolu_orm_macros::Relational;
use toolu_orm_query::relational_builder::RelationalQuery;
use toolu_orm_query::select::RelationColumn;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct FileRow {
  id: String,
  payload: Option<Vec<u8>>,
}

#[derive(Relational, Debug)]
#[relational(table = "owners")]
struct OwnerWithFiles {
  id: String,
  name: String,
  #[has_many(table = "files", foreign_key = "owner_id", columns = ["id", "payload"])]
  files: Vec<FileRow>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct OwnerRow {
  id: String,
  avatar: Option<Vec<u8>>,
}

#[derive(Relational, Debug)]
#[relational(table = "files")]
struct FileWithOwner {
  id: String,
  #[belongs_to(table = "owners", foreign_key = "id", columns = ["id", "avatar"])]
  owner: Option<OwnerRow>,
}

fn file_columns() -> Vec<RelationColumn> {
  vec![RelationColumn::new("id"), RelationColumn::binary("payload")]
}

fn owner_columns() -> Vec<RelationColumn> {
  vec![RelationColumn::new("id"), RelationColumn::binary("avatar")]
}

#[tokio::test]
async fn with_many_round_trips_nonempty_empty_and_null_blobs() -> TestResult {
  let conn = db::setup_db().await?;
  // What the driver itself reads for f1, so the relation is compared against the
  // stored bytes rather than only against the seed constant.
  let direct: Vec<u8> = conn
    .query("SELECT payload FROM files WHERE id = 'f1'", ())
    .await?
    .next()
    .await?
    .ok_or("f1 must exist")?
    .get(0)?;
  let q = RelationalQuery::<(OwnerWithFiles,)>::new("owners", OwnerWithFiles::SCALAR_COLUMNS)
    .with_many_columns::<FileRow>("files", "files", "id", "owner_id", &file_columns());

  let mut rows = conn.query(&q.to_sql_sqlite(), ()).await?;
  let mut seen_ann = false;
  let mut seen_bea = false;
  while let Some(row) = rows.next().await? {
    let id: String = row.get(0)?;
    let name: String = row.get(1)?;
    let files_json: String = row.get(2)?;
    let values = vec![
      serde_json::json!(id),
      serde_json::json!(name),
      q.decode_relation_json("files", &files_json)?,
    ];
    let owner = OwnerWithFiles::from_relational_values(&values)?;
    assert_eq!(
      owner.name, name,
      "scalar columns decode alongside the relation"
    );

    match owner.id.as_str() {
      "o1" => {
        let mut by_id: Vec<&FileRow> = owner.files.iter().collect();
        by_id.sort_by(|a, b| a.id.cmp(&b.id));
        assert_eq!(by_id.len(), 3, "o1 owns f1, f2 and f3: {by_id:?}");
        assert_eq!(
          by_id.first().and_then(|f| f.payload.as_deref()),
          Some(db::PAYLOAD),
          "a nonempty blob must survive byte for byte"
        );
        assert_eq!(
          by_id.first().and_then(|f| f.payload.as_deref()),
          Some(direct.as_slice()),
          "the relation must agree with the driver's own read of the same row"
        );
        assert_eq!(
          by_id.get(1).and_then(|f| f.payload.as_deref()),
          Some(&[][..]),
          "an empty blob must decode to empty bytes, not NULL"
        );
        assert_eq!(
          by_id.get(2).map(|f| f.payload.clone()),
          Some(None),
          "a NULL blob must decode to None, not empty bytes"
        );
        seen_ann = true;
      },
      "o2" => {
        assert!(
          owner.files.is_empty(),
          "o2 owns no files: {:?}",
          owner.files
        );
        seen_bea = true;
      },
      other => return Err(format!("unexpected owner id: {other}").into()),
    }
  }
  assert!(seen_ann && seen_bea, "both owners must be visited");
  Ok(())
}

#[tokio::test]
async fn with_one_round_trips_a_binary_column_and_reports_an_orphan() -> TestResult {
  let conn = db::setup_db().await?;
  let q = RelationalQuery::<(FileWithOwner,)>::new("files", FileWithOwner::SCALAR_COLUMNS)
    .with_one_columns::<OwnerRow>("owner", "owners", "owner_id", "id", &owner_columns());

  let mut rows = conn.query(&q.to_sql_sqlite(), ()).await?;
  let mut checked_owned = false;
  let mut checked_orphan = false;
  while let Some(row) = rows.next().await? {
    let id: String = row.get(0)?;
    let owner_json: Option<String> = row.get(1)?;
    let owner_value = match owner_json {
      Some(text) => q.decode_relation_json("owner", &text)?,
      None => serde_json::Value::Null,
    };
    let file = FileWithOwner::from_relational_values(&[serde_json::json!(id), owner_value])?;

    if file.id == "f1" {
      let owner = file.owner.as_ref().ok_or("f1 must have an owner")?;
      assert_eq!(owner.id, "o1");
      assert_eq!(
        owner.avatar.as_deref(),
        Some(db::PAYLOAD),
        "the parent's binary column must survive byte for byte"
      );
      checked_owned = true;
    } else if file.id == "f4" {
      assert!(file.owner.is_none(), "f4 has no owner: {:?}", file.owner);
      checked_orphan = true;
    }
  }
  assert!(checked_owned && checked_orphan);
  Ok(())
}

#[tokio::test]
async fn with_one_reports_a_null_binary_column_on_a_matched_parent() -> TestResult {
  let conn = db::setup_db().await?;
  conn
    .execute("UPDATE files SET owner_id = 'o2' WHERE id = 'f3'", ())
    .await?;
  let q = RelationalQuery::<(FileWithOwner,)>::new("files", &["id"]).with_one_columns::<OwnerRow>(
    "owner",
    "owners",
    "owner_id",
    "id",
    &owner_columns(),
  );

  let sql = format!("{} WHERE \"files\".\"owner_id\" = 'o2'", q.to_sql_sqlite());
  let mut rows = conn.query(&sql, ()).await?;
  let row = rows.next().await?.ok_or("o2 must match a file")?;
  let text: String = row.get(1)?;
  let decoded = q.decode_relation_json("owner", &text)?;

  assert_eq!(
    decoded,
    serde_json::json!(["o2", null]),
    "a NULL binary column stays null instead of collapsing to empty bytes"
  );
  let file = FileWithOwner::from_relational_values(&[serde_json::json!("f3"), decoded])?;
  let owner = file.owner.as_ref().ok_or("o2 must decode")?;
  assert_eq!(owner.id, "o2");
  assert!(owner.avatar.is_none());
  Ok(())
}

#[tokio::test]
async fn an_undeclared_blob_column_is_still_rejected_by_libsql() -> TestResult {
  let conn = db::setup_db().await?;
  let sql = RelationalQuery::<(OwnerWithFiles,)>::new("owners", &["id"])
    .with_many::<FileRow>("files", "files", "id", "owner_id", &["id", "payload"])
    .to_sql_sqlite();

  let mut rows = conn.query(&sql, ()).await?;
  let err = rows
    .next()
    .await
    .err()
    .ok_or("an undeclared BLOB projection must fail at the database")?;
  assert!(
    err.to_string().contains("JSON cannot hold BLOB values"),
    "expected SQLite's BLOB refusal, got: {err}"
  );
  Ok(())
}
