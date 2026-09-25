use super::support::{names, open, pairs, quoted, refusal, trial};
use std::error::Error;

#[test]
fn ddl_and_constraint_boundaries_persist_after_reopen() -> Result<(), Box<dyn Error>> {
  let directory = tempfile::tempdir()?;
  let connection = open(directory.path())?;
  trial(
    &connection,
    "create",
    "CREATE TABLE lance_probe.main.items (id BIGINT, label VARCHAR, vec FLOAT[2])",
  );
  trial(
    &connection,
    "insert",
    "INSERT INTO lance_probe.main.items VALUES (1, 'one', [0.0, 0.0]), (2, 'two', [1.0, 0.0])",
  );
  trial(
    &connection,
    "add",
    "ALTER TABLE lance_probe.main.items ADD COLUMN score BIGINT DEFAULT 7",
  );
  trial(
    &connection,
    "rename",
    "ALTER TABLE lance_probe.main.items RENAME COLUMN label TO title",
  );
  refusal(
    &connection,
    "ALTER TABLE lance_probe.main.items ALTER COLUMN id TYPE DOUBLE",
    "Cannot cast column \"id\" from Int64 to Float64",
  );
  refusal(
    &connection,
    "ALTER TABLE lance_probe.main.items ALTER COLUMN vec TYPE FLOAT[3]",
    "Cannot cast column \"vec\"",
  );
  trial(
    &connection,
    "drop column",
    "ALTER TABLE lance_probe.main.items DROP COLUMN score",
  );
  let index_sql = format!(
    "CREATE INDEX id_idx ON {} (id) USING BTREE",
    quoted(&directory.path().join("items.lance"))?
  );
  trial(&connection, "create index", &index_sql);
  trial(
    &connection,
    "duplicate with btree",
    "INSERT INTO lance_probe.main.items VALUES (1, 'duplicate', [2.0, 0.0])",
  );
  for (label, declaration) in [
    ("pk", "id BIGINT PRIMARY KEY"),
    ("unique", "id BIGINT UNIQUE"),
    ("not null", "id BIGINT NOT NULL"),
    ("check", "id BIGINT CHECK (id > 0)"),
    ("fk", "id BIGINT REFERENCES lance_probe.main.items(id)"),
  ] {
    let table = label.replace(' ', "_");
    refusal(
      &connection,
      &format!("CREATE TABLE lance_probe.main.{table}_case ({declaration})"),
      if label == "fk" {
        "FOREIGN KEY constraints cannot be defined cross-database"
      } else {
        "Lance CREATE TABLE does not support constraints"
      },
    );
    assert!(
      !directory
        .path()
        .join(format!("{table}_case.lance"))
        .exists()
    );
  }
  refusal(
    &connection,
    "CREATE TABLE lance_probe.main.fk_local_case (id BIGINT REFERENCES items(id))",
    "Table with name items does not exist",
  );
  trial(&connection, "use lance", "USE lance_probe");
  refusal(
    &connection,
    "CREATE TABLE fk_within_case (id BIGINT REFERENCES items(id))",
    "there is no primary key or unique constraint",
  );
  refusal(
    &connection,
    &format!(
      "CREATE UNIQUE INDEX unique_idx ON {} (id) USING BTREE",
      quoted(&directory.path().join("items.lance"))?
    ),
    "syntax error at or near \"USING\"",
  );
  trial(
    &connection,
    "create drop case",
    "CREATE TABLE lance_probe.main.drop_case (id BIGINT)",
  );
  trial(
    &connection,
    "drop table",
    "DROP TABLE lance_probe.main.drop_case",
  );
  drop(connection);
  let reopened = open(directory.path())?;
  assert_eq!(
    names(&reopened, "DESCRIBE lance_probe.main.items")?,
    ["id", "title", "vec"]
  );
  assert_eq!(
    names(
      &reopened,
      &format!(
        "SHOW INDEXES ON {}",
        quoted(&directory.path().join("items.lance"))?
      )
    )?,
    ["id_idx"]
  );
  let duplicate_count: i64 = reopened.query_row(
    "SELECT count(*) FROM lance_probe.main.items WHERE id = 1",
    [],
    |row| row.get(0),
  )?;
  assert_eq!(duplicate_count, 2);
  let still_two_dimensional = format!(
    "SELECT id, _distance FROM lance_vector_search({}, 'vec', [0.0, 0.0]::FLOAT[2], k = 3, use_index = false) ORDER BY _distance",
    quoted(&directory.path().join("items.lance"))?
  );
  let distances: Vec<f64> = pairs(&reopened, &still_two_dimensional)?
    .into_iter()
    .map(|(_, distance)| distance)
    .collect();
  assert_eq!(distances, [0.0, 1.0, 4.0]);
  trial(
    &reopened,
    "drop index",
    &format!(
      "DROP INDEX id_idx ON {}",
      quoted(&directory.path().join("items.lance"))?
    ),
  );
  assert!(
    names(
      &reopened,
      &format!(
        "SHOW INDEXES ON {}",
        quoted(&directory.path().join("items.lance"))?
      )
    )?
    .is_empty()
  );
  refusal(
    &reopened,
    "SELECT * FROM lance_probe.main.drop_case",
    "Table with name drop_case does not exist",
  );
  Ok(())
}
