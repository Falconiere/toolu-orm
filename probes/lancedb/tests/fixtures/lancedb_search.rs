use super::support::{names, open, pairs, quoted, refusal};
use duckdb::Connection;
use std::error::Error;

fn seed_search(connection: &Connection) -> duckdb::Result<()> {
  connection.execute_batch(
    "CREATE TABLE lance_probe.main.search_items (id BIGINT, tag VARCHAR, body VARCHAR, vec FLOAT[2]);
     INSERT INTO lance_probe.main.search_items VALUES
       (1, 'skip', 'red fox', [0.0, 0.0]),
       (2, 'keep', 'blue fox', [1.0, 0.0]),
       (3, 'keep', 'blue bird', [2.0, 0.0]);",
  )
}

#[test]
fn vector_and_fts_search_filters_have_distinct_rankings() -> Result<(), Box<dyn Error>> {
  let directory = tempfile::tempdir()?;
  let connection = open(directory.path())?;
  seed_search(&connection)?;
  let dataset = quoted(&directory.path().join("search_items.lance"))?;

  let raw_vector = format!(
    "SELECT id, _distance FROM lance_vector_search({dataset}, 'vec', [0.0, 0.0]::FLOAT[2], k = 2, use_index = false) ORDER BY _distance"
  );
  assert_eq!(pairs(&connection, &raw_vector)?, [(1, 0.0), (2, 1.0)]);
  let outer_where = format!(
    "SELECT id, _distance FROM lance_vector_search({dataset}, 'vec', [0.0, 0.0]::FLOAT[2], k = 2, use_index = false) WHERE tag = 'keep' ORDER BY _distance"
  );
  assert_eq!(pairs(&connection, &outer_where)?, [(2, 1.0)]);
  let namespace_prefilter = "SELECT id, _distance FROM lance_vector_search('lance_probe.main.search_items', 'vec', [0.0, 0.0]::FLOAT[2], k = 2, use_index = false, filter = 'tag = ''keep''', prefilter = true) ORDER BY _distance";
  assert_eq!(
    pairs(&connection, namespace_prefilter)?,
    [(2, 1.0), (3, 4.0)]
  );
  let path_prefilter = format!(
    "SELECT id, _distance FROM lance_vector_search({dataset}, 'vec', [0.0, 0.0]::FLOAT[2], k = 2, use_index = false, filter = 'tag = ''keep''', prefilter = true)"
  );
  let error = pairs(&connection, &path_prefilter).expect_err("path vector filter must fail");
  assert!(
    error
      .to_string()
      .contains("filter parameter is only supported for namespace-backed tables")
  );
  let wrong_dimension = format!(
    "SELECT id, _distance FROM lance_vector_search({dataset}, 'vec', [0.0, 0.0, 0.0]::FLOAT[3], k = 2, use_index = false)"
  );
  let error = pairs(&connection, &wrong_dimension).expect_err("wrong dimension must fail");
  assert!(
    error
      .to_string()
      .contains("query dim(3) doesn't match the column vec vector dim(2)")
  );

  let fts = format!(
    "SELECT id, _score FROM lance_fts({dataset}, 'body', 'fox', k = 2) ORDER BY _score DESC"
  );
  let unindexed = pairs(&connection, &fts)?;
  let mut ids: Vec<i64> = unindexed.iter().map(|(id, _)| *id).collect();
  ids.sort_unstable();
  assert_eq!(ids, [1, 2]);
  assert!(unindexed.iter().all(|(_, score)| *score > 0.0));
  let namespace_fts = "SELECT id, _score FROM lance_fts('lance_probe.main.search_items', 'body', 'fox', k = 2, filter = 'tag = ''keep''', prefilter = true) ORDER BY _score DESC";
  let filtered = pairs(&connection, namespace_fts)?;
  assert_eq!(filtered.len(), 1);
  assert_eq!(filtered.first().map(|(id, _)| *id), Some(2));
  assert!(filtered.first().is_some_and(|(_, score)| *score > 0.0));
  let path_fts = format!(
    "SELECT id, _score FROM lance_fts({dataset}, 'body', 'fox', k = 2, filter = 'tag = ''keep''', prefilter = true)"
  );
  let error = pairs(&connection, &path_fts).expect_err("path FTS filter must fail");
  assert!(
    error
      .to_string()
      .contains("filter parameter is only supported for namespace-backed tables")
  );

  connection.execute_batch(&format!(
    "CREATE INDEX body_idx ON {dataset} (body) USING INVERTED"
  ))?;
  drop(connection);
  let reopened = open(directory.path())?;
  assert_eq!(
    names(&reopened, &format!("SHOW INDEXES ON {dataset}"))?,
    ["body_idx"]
  );
  let indexed = pairs(&reopened, &fts)?;
  let mut indexed_ids: Vec<i64> = indexed.iter().map(|(id, _)| *id).collect();
  indexed_ids.sort_unstable();
  assert_eq!(indexed_ids, [1, 2]);
  assert!(indexed.iter().all(|(_, score)| *score > 0.0));
  Ok(())
}

#[test]
fn dml_commit_and_rollback_persist_but_ddl_in_transaction_refuses() -> Result<(), Box<dyn Error>> {
  let directory = tempfile::tempdir()?;
  let connection = open(directory.path())?;
  connection.execute_batch(
    "CREATE TABLE lance_probe.main.tx_items (id BIGINT);
     INSERT INTO lance_probe.main.tx_items VALUES (1);",
  )?;
  connection.execute_batch(
    "BEGIN;
     INSERT INTO lance_probe.main.tx_items VALUES (2);
     ROLLBACK;
     BEGIN;
     INSERT INTO lance_probe.main.tx_items VALUES (3);
     COMMIT;",
  )?;
  connection.execute_batch("BEGIN; INSERT INTO lance_probe.main.tx_items VALUES (4)")?;
  refusal(
    &connection,
    "ALTER TABLE lance_probe.main.tx_items ADD COLUMN added BIGINT",
    "Lance DDL does not support explicit transactions yet",
  );
  connection.execute_batch("ROLLBACK")?;
  drop(connection);

  let reopened = open(directory.path())?;
  assert_eq!(
    names(&reopened, "DESCRIBE lance_probe.main.tx_items")?,
    ["id"]
  );
  for (id, expected) in [(1, 1), (2, 0), (3, 1), (4, 0)] {
    let count: i64 = reopened.query_row(
      "SELECT count(*) FROM lance_probe.main.tx_items WHERE id = ?",
      [id],
      |row| row.get(0),
    )?;
    assert_eq!(count, expected, "id={id}");
  }
  Ok(())
}
