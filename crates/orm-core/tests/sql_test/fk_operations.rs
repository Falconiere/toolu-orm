use toolu_orm_core::column::ForeignKeyAction;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::diff::Operation;
use toolu_orm_core::snapshot::ForeignKeyDef;
use toolu_orm_core::sql::generate_sql_for;

#[test]
fn add_foreign_key_postgres() {
  let ops = vec![Operation::AddForeignKey {
    table: "posts".to_owned(),
    fk: ForeignKeyDef {
      name: "fk_posts_author".to_owned(),
      columns: vec!["author_id".to_owned()],
      references_table: "users".to_owned(),
      references_columns: vec!["id".to_owned()],
      on_delete: Some(ForeignKeyAction::Cascade),
      on_update: None,
    },
  }];
  let sql = generate_sql_for(&ops, Dialect::Postgres);
  assert!(
    sql.contains("ALTER TABLE \"posts\" ADD CONSTRAINT \"fk_posts_author\""),
    "sql: {sql}"
  );
  assert!(
    sql.contains("FOREIGN KEY (\"author_id\") REFERENCES \"users\" (\"id\")"),
    "sql: {sql}"
  );
  assert!(sql.contains("ON DELETE CASCADE"), "sql: {sql}");
}

#[test]
fn drop_foreign_key_postgres() {
  let ops = vec![Operation::DropForeignKey {
    table: "posts".to_owned(),
    name: "fk_posts_author".to_owned(),
  }];
  let sql = generate_sql_for(&ops, Dialect::Postgres);
  assert!(
    sql.contains("ALTER TABLE \"posts\" DROP CONSTRAINT IF EXISTS \"fk_posts_author\""),
    "sql: {sql}"
  );
}
