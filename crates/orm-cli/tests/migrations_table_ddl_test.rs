use toolu_orm_cli::migrate::migrations_table_ddl;
use toolu_orm_core::dialect::Dialect;

#[test]
fn migrations_table_ddl_sqlite() {
  let ddl = migrations_table_ddl(Dialect::Sqlite);
  assert!(ddl.contains("INTEGER PRIMARY KEY AUTOINCREMENT"));
  assert!(ddl.contains("DEFAULT (unixepoch())"));
  assert!(!ddl.contains("SERIAL"));
}

#[test]
fn migrations_table_ddl_postgres() {
  let ddl = migrations_table_ddl(Dialect::Postgres);
  assert!(ddl.contains("SERIAL PRIMARY KEY"));
  assert!(ddl.contains("extract(epoch from now())::bigint"));
  assert!(!ddl.contains("AUTOINCREMENT"));
  assert!(!ddl.contains("unixepoch()"));
}
