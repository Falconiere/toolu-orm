//! Descending index columns: SQL render, serde wire shape, and diff drop+create.

use toolu_orm_core::column::ColumnType;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::diff::{diff, Operation};
use toolu_orm_core::index::{IndexColumn, IndexDef};
use toolu_orm_core::schema::SchemaRegistry;
use toolu_orm_core::snapshot::Snapshot;
use toolu_orm_core::sql::generate_sql_for;
use toolu_orm_core::table::{TableDef, TableKind};

fn col(name: &str) -> toolu_orm_core::column::ColumnDef {
  toolu_orm_core::column::ColumnDef {
    name: name.to_owned(),
    column_type: ColumnType::Text,
    primary_key: name == "id",
    not_null: name == "id",
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
    name: "eval_runs".to_owned(),
    columns: vec![col("id"), col("at")],
    indexes: vec![index],
    primary_key: Vec::new(),
    strict: false,
    kind: TableKind::Ordinary,
  }
}

#[test]
fn create_index_sql_renders_desc_suffix() {
  let ops = vec![Operation::CreateIndex {
    table: "eval_runs".to_owned(),
    index: IndexDef {
      name: "idx_eval_runs_at".to_owned(),
      columns: vec![IndexColumn::desc("at")],
      unique: false,
      where_clause: None,
    },
  }];
  let sql = generate_sql_for(&ops, Dialect::Sqlite);
  assert!(
    sql.contains(r#"CREATE INDEX IF NOT EXISTS "idx_eval_runs_at" ON "eval_runs" ("at" DESC)"#),
    "got: {sql}"
  );
  let pg = generate_sql_for(&ops, Dialect::Postgres);
  assert!(
    pg.contains(r#"CREATE INDEX IF NOT EXISTS "idx_eval_runs_at" ON "eval_runs" ("at" DESC)"#),
    "got: {pg}"
  );
}

#[test]
fn create_index_sql_keeps_ascending_without_desc() {
  let ops = vec![Operation::CreateIndex {
    table: "eval_runs".to_owned(),
    index: IndexDef {
      name: "idx_eval_runs_at".to_owned(),
      columns: vec![IndexColumn::new("at")],
      unique: false,
      where_clause: None,
    },
  }];
  let sql = generate_sql_for(&ops, Dialect::Sqlite);
  assert!(sql.contains(r#"ON "eval_runs" ("at")"#), "got: {sql}");
  assert!(!sql.contains("DESC"), "got: {sql}");
}

#[test]
fn index_column_deserializes_legacy_string_list() -> Result<(), Box<dyn std::error::Error>> {
  let json = r#"{
    "name": "idx_eval_runs_at",
    "columns": ["at", "id"],
    "unique": false
  }"#;
  let index: IndexDef = serde_json::from_str(json)?;
  assert_eq!(
    index.columns,
    vec![IndexColumn::new("at"), IndexColumn::new("id")]
  );
  Ok(())
}

#[test]
fn index_column_roundtrips_desc_object() -> Result<(), Box<dyn std::error::Error>> {
  let index = IndexDef {
    name: "idx_eval_runs_at".to_owned(),
    columns: vec![IndexColumn::desc("at"), IndexColumn::new("id")],
    unique: false,
    where_clause: None,
  };
  let json = serde_json::to_string(&index)?;
  assert!(json.contains(r#""desc":true"#), "got: {json}");
  assert!(
    !json.contains(r#""desc":false"#),
    "ascending should omit desc: {json}"
  );
  let parsed: IndexDef = serde_json::from_str(&json)?;
  assert_eq!(parsed, index);
  Ok(())
}

#[test]
fn diff_direction_change_is_drop_then_create() -> Result<(), Box<dyn std::error::Error>> {
  let old = Snapshot::from_registry(&SchemaRegistry::from_tables(vec![table_with_index(
    IndexDef {
      name: "idx_eval_runs_at".to_owned(),
      columns: vec![IndexColumn::new("at")],
      unique: false,
      where_clause: None,
    },
  )]));
  let new_reg = SchemaRegistry::from_tables(vec![table_with_index(IndexDef {
    name: "idx_eval_runs_at".to_owned(),
    columns: vec![IndexColumn::desc("at")],
    unique: false,
    where_clause: None,
  })]);
  let ops = diff(&old, &new_reg)?;
  assert!(
    ops
      .iter()
      .any(|op| matches!(op, Operation::DropIndex { name } if name == "idx_eval_runs_at")),
    "expected DropIndex, got: {ops:?}"
  );
  assert!(
    ops.iter().any(|op| matches!(
      op,
      Operation::CreateIndex { table, index }
        if table == "eval_runs"
          && index.name == "idx_eval_runs_at"
          && index.columns == vec![IndexColumn::desc("at")]
    )),
    "expected CreateIndex with DESC, got: {ops:?}"
  );
  Ok(())
}
