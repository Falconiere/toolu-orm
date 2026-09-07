use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::diff::Operation;
use toolu_orm_core::sql::generate_sql_for;

#[test]
fn add_check_constraint_postgres() {
  let ops = vec![Operation::AddCheckConstraint {
    table: "items".to_owned(),
    name: "chk_status".to_owned(),
    expr: "\"status\" IN ('draft', 'active')".to_owned(),
  }];
  let sql = generate_sql_for(&ops, Dialect::Postgres);
  assert!(
    sql.contains("ALTER TABLE \"items\" ADD CONSTRAINT \"chk_status\" CHECK"),
    "sql: {sql}"
  );
}

#[test]
fn drop_check_constraint_postgres() {
  let ops = vec![Operation::DropCheckConstraint {
    table: "items".to_owned(),
    name: "chk_status".to_owned(),
  }];
  let sql = generate_sql_for(&ops, Dialect::Postgres);
  assert!(
    sql.contains("ALTER TABLE \"items\" DROP CONSTRAINT IF EXISTS \"chk_status\""),
    "sql: {sql}"
  );
}
