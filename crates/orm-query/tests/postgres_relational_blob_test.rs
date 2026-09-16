//! Relational loads over a real `bytea` column on the live Postgres: the same
//! declaration and decoding as the SQLite lanes, and proof that the decoded
//! value is a JSON array of bytes rather than bytea's `\x…` text rendering.
//!
//! Needs `docker compose -f docker-compose.test.yaml up -d --wait` and
//! `TEST_DB_PORT=5434` locally; compiles only via the postgres lane.

#[path = "fixtures/postgres_db.rs"]
pub mod pg;

use std::collections::BTreeMap;

use serde::Deserialize;
use toolu_orm_core::relational_row::FromRelationalRow;
use toolu_orm_macros::Relational;
use toolu_orm_query::relational_builder::RelationalQuery;
use toolu_orm_query::select::RelationColumn;

use pg::{client, TestResult};

/// `files.payload` for f1.
const PAYLOAD: &[u8] = &[0x01, 0x02, 0xFF, 0x00, 0x7F];
/// `owners.avatar` for o1 — deliberately a different byte order than
/// [`PAYLOAD`], so a decoded avatar can never pass as a decoded payload.
const AVATAR: &[u8] = &[0x01, 0x02, 0x00, 0xFF, 0x7F];

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct FileRow {
  id: String,
  payload: Option<Vec<u8>>,
}

#[derive(Relational, Debug)]
#[relational(table = "owners")]
struct OwnerWithFiles {
  id: String,
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

/// o1 Ann owns f1 (nonempty), f2 (empty) and f3 (NULL); o2 Bea owns none; f4 has
/// no owner. `owners.avatar` is nonempty for o1 and NULL for o2.
async fn seeded(schema: &str) -> Result<tokio_postgres::Client, Box<dyn std::error::Error>> {
  let client = client(schema).await?;
  client
    .batch_execute(
      "CREATE TABLE owners (id TEXT PRIMARY KEY, avatar bytea);
       CREATE TABLE files (id TEXT PRIMARY KEY, owner_id TEXT REFERENCES owners(id), payload bytea);
       INSERT INTO owners (id, avatar) VALUES ('o1', '\\x010200ff7f'), ('o2', NULL);
       INSERT INTO files (id, owner_id, payload) VALUES
         ('f1', 'o1', '\\x0102ff007f'), ('f2', 'o1', ''::bytea), ('f3', 'o1', NULL),
         ('f4', NULL, '\\xab');",
    )
    .await?;
  Ok(client)
}

fn file_columns() -> Vec<RelationColumn> {
  vec![RelationColumn::new("id"), RelationColumn::binary("payload")]
}

fn owner_columns() -> Vec<RelationColumn> {
  vec![RelationColumn::new("id"), RelationColumn::binary("avatar")]
}

#[tokio::test]
async fn with_many_round_trips_nonempty_empty_and_null_bytea() -> TestResult {
  let client = seeded("q_pg_rel_blob_many").await?;
  // What the driver itself reads for f1, so the relation is compared against the
  // stored bytes rather than only against the seed constant.
  let direct: Vec<u8> = client
    .query_one("SELECT payload FROM files WHERE id = 'f1'", &[])
    .await?
    .try_get(0)?;
  let q = RelationalQuery::<(OwnerWithFiles,)>::new("owners", OwnerWithFiles::SCALAR_COLUMNS)
    .with_many_columns::<FileRow>("files", "files", "id", "owner_id", &file_columns());

  let sql = q.to_sql_postgres();
  assert!(
    sql.contains(r#"encode("files_sub"."payload", 'hex')"#),
    "binary column should be hex-encoded: {sql}"
  );

  let mut by_id = BTreeMap::new();
  for row in client.query(&sql, &[]).await? {
    let id: String = row.try_get(0)?;
    let column: Option<serde_json::Value> = row.try_get(1)?;
    let decoded = q.decode_relation_value("files", &column.unwrap_or(serde_json::Value::Null))?;
    let owner = OwnerWithFiles::from_relational_values(&[serde_json::json!(id), decoded])?;
    by_id.insert(owner.id.clone(), owner);
  }

  let ann = by_id.get("o1").ok_or("o1 missing")?;
  let mut files: Vec<&FileRow> = ann.files.iter().collect();
  files.sort_by(|a, b| a.id.cmp(&b.id));
  assert_eq!(files.len(), 3, "o1 owns f1, f2 and f3: {files:?}");
  assert_eq!(
    files.first().and_then(|f| f.payload.as_deref()),
    Some(PAYLOAD),
    "a nonempty bytea must survive byte for byte"
  );
  assert_eq!(
    files.first().and_then(|f| f.payload.as_deref()),
    Some(direct.as_slice()),
    "the relation must agree with the driver's own read of the same row"
  );
  assert_eq!(
    files.get(1).and_then(|f| f.payload.as_deref()),
    Some(&[][..]),
    "an empty bytea must decode to empty bytes, not NULL"
  );
  assert_eq!(
    files.get(2).map(|f| f.payload.clone()),
    Some(None),
    "a NULL bytea must decode to None, not empty bytes"
  );

  let bea = by_id.get("o2").ok_or("o2 missing")?;
  assert!(bea.files.is_empty(), "o2 owns no files: {:?}", bea.files);
  Ok(())
}

#[tokio::test]
async fn with_one_round_trips_bytea_and_reports_an_orphan() -> TestResult {
  let client = seeded("q_pg_rel_blob_one").await?;
  let q = RelationalQuery::<(FileWithOwner,)>::new("files", FileWithOwner::SCALAR_COLUMNS)
    .with_one_columns::<OwnerRow>("owner", "owners", "owner_id", "id", &owner_columns());

  let mut by_id = BTreeMap::new();
  for row in client.query(&q.to_sql_postgres(), &[]).await? {
    let id: String = row.try_get(0)?;
    let column: Option<serde_json::Value> = row.try_get(1)?;
    let decoded = q.decode_relation_value("owner", &column.unwrap_or(serde_json::Value::Null))?;
    let file = FileWithOwner::from_relational_values(&[serde_json::json!(id), decoded])?;
    by_id.insert(file.id.clone(), file);
  }

  let matched = by_id.get("f1").ok_or("f1 missing")?;
  let owner = matched.owner.as_ref().ok_or("f1 must have an owner")?;
  assert_eq!(owner.id, "o1");
  assert_eq!(
    owner.avatar.as_deref(),
    Some(AVATAR),
    "the parent's bytea column must survive byte for byte"
  );

  let orphan = by_id.get("f4").ok_or("f4 missing")?;
  assert!(
    orphan.owner.is_none(),
    "f4 has no owner: {:?}",
    orphan.owner
  );

  let null_avatar = by_id.get("f3").ok_or("f3 missing")?;
  assert_eq!(
    null_avatar.owner.as_ref().map(|o| o.avatar.clone()),
    Some(Some(AVATAR.to_vec())),
    "f3 still points at o1"
  );
  Ok(())
}

#[tokio::test]
async fn an_undeclared_bytea_column_arrives_as_escape_text() -> TestResult {
  let client = seeded("q_pg_rel_blob_raw").await?;
  let undeclared = RelationalQuery::<(OwnerWithFiles,)>::new("owners", &["id"])
    .with_many::<FileRow>("files", "files", "id", "owner_id", &["id", "payload"]);
  let declared = RelationalQuery::<(OwnerWithFiles,)>::new("owners", &["id"])
    .with_many_columns::<FileRow>("files", "files", "id", "owner_id", &file_columns());

  let raw: Option<serde_json::Value> = client
    .query_one(
      &format!(
        "{} WHERE \"owners\".\"id\" = 'o1'",
        undeclared.to_sql_postgres()
      ),
      &[],
    )
    .await?
    .try_get(1)?;
  let raw_first = raw
    .as_ref()
    .and_then(|v| v.as_array())
    .and_then(|rows| {
      rows
        .iter()
        .find(|row| row.get(0) == Some(&serde_json::json!("f1")))
    })
    .and_then(|row| row.get(1))
    .ok_or("f1 missing from the undeclared projection")?;
  assert!(
    raw_first.as_str().is_some_and(|s| s.starts_with("\\x")),
    "without a declaration Postgres renders bytea as escape text: {raw_first}"
  );

  let decoded_column: Option<serde_json::Value> = client
    .query_one(
      &format!(
        "{} WHERE \"owners\".\"id\" = 'o1'",
        declared.to_sql_postgres()
      ),
      &[],
    )
    .await?
    .try_get(1)?;
  let decoded =
    declared.decode_relation_value("files", &decoded_column.unwrap_or(serde_json::Value::Null))?;
  let decoded_first = decoded
    .as_array()
    .and_then(|rows| {
      rows
        .iter()
        .find(|row| row.get(0) == Some(&serde_json::json!("f1")))
    })
    .and_then(|row| row.get(1))
    .ok_or("f1 missing from the declared projection")?;
  assert_eq!(
    decoded_first,
    &serde_json::json!([1, 2, 255, 0, 127]),
    "the declared projection decodes to bytes, never to text"
  );
  Ok(())
}
