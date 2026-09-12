//! Partial indexes: SQL render, serde round-trip of legacy snapshots, and
//! diff drop+create when the WHERE predicate changes.

use toolu_orm_core::column::{ColumnDef, ColumnType};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::diff::{diff, Operation};
use toolu_orm_core::index::{IndexColumn, IndexDef};
use toolu_orm_core::schema::SchemaRegistry;
use toolu_orm_core::snapshot::Snapshot;
use toolu_orm_core::sql::generate_sql_for;
use toolu_orm_core::table::TableDef;

fn col(name: &str, ct: ColumnType, pk: bool, nn: bool) -> ColumnDef {
  ColumnDef {
    name: name.to_owned(),
    column_type: ct,
    primary_key: pk,
    not_null: nn,
    default: None,
    unique: false,
    references: None,
    on_delete: None,
    on_update: None,
    check: None,
    unindexed: false,
    autoincrement: false,
  }
}

fn table_with_index(index: IndexDef) -> TableDef {
  TableDef {
    name: "memories".to_owned(),
    columns: vec![
      col("id", ColumnType::Text, true, true),
      col("repo", ColumnType::Text, false, true),
      col("deleted_at", ColumnType::Timestamp, false, false),
    ],
    indexes: vec![index],
    primary_key: Vec::new(),
    strict: false,
    kind: toolu_orm_core::table::TableKind::Ordinary,
  }
}

fn partial_index() -> IndexDef {
  IndexDef {
    name: "idx_memories_repo".to_owned(),
    columns: vec![IndexColumn::new("repo")],
    unique: false,
    where_clause: Some("deleted_at IS NULL".to_owned()),
  }
}

#[test]
fn create_index_sql_appends_where_predicate() {
  let ops = vec![Operation::CreateIndex {
    table: "memories".to_owned(),
    index: partial_index(),
  }];
  for dialect in [Dialect::Sqlite, Dialect::Postgres] {
    let sql = generate_sql_for(&ops, dialect);
    assert!(
      sql.contains(
        r#"CREATE INDEX IF NOT EXISTS "idx_memories_repo" ON "memories" ("repo") WHERE deleted_at IS NULL;"#
      ),
      "dialect={dialect:?} sql={sql}"
    );
  }
}

#[test]
fn create_unique_index_sql_appends_where_predicate() {
  let ops = vec![Operation::CreateIndex {
    table: "memories".to_owned(),
    index: IndexDef {
      name: "idx_memories_repo_unique".to_owned(),
      columns: vec![IndexColumn::new("repo")],
      unique: true,
      where_clause: Some("deleted_at IS NULL".to_owned()),
    },
  }];
  let sql = generate_sql_for(&ops, Dialect::Sqlite);
  assert!(
    sql.contains(
      r#"CREATE UNIQUE INDEX IF NOT EXISTS "idx_memories_repo_unique" ON "memories" ("repo") WHERE deleted_at IS NULL;"#
    ),
    "got: {sql}"
  );
}

#[test]
fn legacy_index_json_without_where_clause_deserializes() -> Result<(), Box<dyn std::error::Error>> {
  let json = r#"{
    "name": "idx_email",
    "columns": ["email"],
    "unique": true
  }"#;
  let index: IndexDef = serde_json::from_str(json)?;
  assert_eq!(index.name, "idx_email");
  assert_eq!(index.columns, vec![IndexColumn::new("email")]);
  assert!(index.unique);
  assert_eq!(index.where_clause, None);

  let with_none = IndexDef {
    name: "idx_email".to_owned(),
    columns: vec![IndexColumn::new("email")],
    unique: true,
    where_clause: None,
  };
  let serialized = serde_json::to_string(&with_none)?;
  assert!(
    !serialized.contains("where_clause"),
    "None where_clause must be omitted: {serialized}"
  );
  let roundtrip: IndexDef = serde_json::from_str(&serialized)?;
  assert_eq!(roundtrip, with_none);
  Ok(())
}

#[test]
fn changed_where_predicate_diffs_as_drop_and_create() -> Result<(), Box<dyn std::error::Error>> {
  let mut old_index = partial_index();
  old_index.where_clause = Some("deleted_at IS NULL".to_owned());
  let old = Snapshot::from_registry(&SchemaRegistry::from_tables(vec![table_with_index(
    old_index,
  )]));

  let mut new_index = partial_index();
  new_index.where_clause = Some("deleted_at IS NULL AND kind = 'note'".to_owned());
  let expected_predicate = new_index.where_clause.clone();
  let new_reg = SchemaRegistry::from_tables(vec![table_with_index(new_index)]);
  let ops = diff(&old, &new_reg)?;

  assert!(
    ops
      .iter()
      .any(|op| matches!(op, Operation::DropIndex { name } if name == "idx_memories_repo")),
    "expected DropIndex, got {ops:?}"
  );
  assert!(
    ops.iter().any(|op| matches!(
      op,
      Operation::CreateIndex { table, index }
        if table == "memories" && index.where_clause == expected_predicate
    )),
    "expected CreateIndex with new predicate, got {ops:?}"
  );
  Ok(())
}
