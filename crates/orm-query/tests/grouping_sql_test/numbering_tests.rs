//! Where the parameters of a grouped statement land: the order they are
//! numbered in, and the placeholder each dialect writes.

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::{OrderBy, Scalar};
use toolu_orm_core::query_column::CommonOps;
use toolu_orm_core::value::Value;
use toolu_orm_query::select::SelectBuilder;

use crate::fixtures::{files, sources, status_counts, LABEL, PATH, SOURCE_ID, SOURCE_PK};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn having_is_numbered_after_the_where_clause() {
  let (sql, params) = status_counts()
    .having(Scalar::count_star().gt(Scalar::bind(1i64)))
    .order_by(OrderBy::alias_desc("n"))
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"SELECT "status", COUNT(*) AS "n" FROM "source_files" "#.to_owned()
      + r#"WHERE "source_files"."source_id" = ?1 "#
      + r#"GROUP BY "source_files"."status" "#
      + r#"HAVING COUNT(*) > ?2 ORDER BY "n" DESC"#
  );
  assert_eq!(
    params,
    vec![Value::Text("s1".to_owned()), Value::Integer(1)]
  );
}

/// Every clause that can bind, binding at once, so the whole numbering order is
/// visible in one statement.
#[test]
fn every_binding_clause_numbers_in_render_order() -> TestResult {
  let f = files();
  let s = sources();
  let prefix = Scalar::func(
    "substr",
    vec![
      f.column(&PATH).scalar(),
      Scalar::bind(1i64),
      Scalar::bind(3i64),
    ],
  )?;

  let (sql, params) = SelectBuilder::from_table(&f)
    .column_scalar(Scalar::bind("s1"), "scope")
    .column_scalar(Scalar::count_star(), "n")
    .join(
      &s,
      s.column(&SOURCE_PK)
        .equals(&f.column(&SOURCE_ID))
        .and(s.column(&LABEL).eq("primary")),
    )
    .filter(f.column(&SOURCE_ID).eq("s1"))
    .group_by_scalar(prefix)
    .having(Scalar::count_star().gt(Scalar::bind(1i64)))
    .order_by(OrderBy::alias_desc("n"))
    .limit(5)
    .offset(1)
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"SELECT ?1 AS "scope", COUNT(*) AS "n" FROM "source_files" AS "f" "#.to_owned()
      + r#"INNER JOIN "sources" AS "s" ON ("s"."id" = "f"."source_id" AND "s"."label" = ?2) "#
      + r#"WHERE "f"."source_id" = ?3 "#
      + r#"GROUP BY substr("f"."path", ?4, ?5) "#
      + r#"HAVING COUNT(*) > ?6 "#
      + r#"ORDER BY "n" DESC LIMIT ?7 OFFSET ?8"#
  );
  assert_eq!(
    params,
    vec![
      Value::Text("s1".to_owned()),
      Value::Text("primary".to_owned()),
      Value::Text("s1".to_owned()),
      Value::Integer(1),
      Value::Integer(3),
      Value::Integer(1),
      Value::Integer(5),
      Value::Integer(1),
    ]
  );
  Ok(())
}

/// The same statement on Postgres: `$N` placeholders, same order, same values.
#[test]
fn the_same_statement_numbers_with_dollar_placeholders_on_postgres() -> TestResult {
  let f = files();
  let s = sources();
  let prefix = Scalar::func(
    "substr",
    vec![
      f.column(&PATH).scalar(),
      Scalar::bind(1i64),
      Scalar::bind(3i64),
    ],
  )?;

  let (sql, params) = SelectBuilder::from_table(&f)
    .column_scalar(Scalar::bind("s1"), "scope")
    .column_scalar(Scalar::count_star(), "n")
    .join(
      &s,
      s.column(&SOURCE_PK)
        .equals(&f.column(&SOURCE_ID))
        .and(s.column(&LABEL).eq("primary")),
    )
    .filter(f.column(&SOURCE_ID).eq("s1"))
    .group_by_scalar(prefix)
    .having(Scalar::count_star().gt(Scalar::bind(1i64)))
    .order_by(OrderBy::alias_desc("n"))
    .limit(5)
    .offset(1)
    .to_sql_for(Dialect::Postgres);

  assert_eq!(
    sql,
    r#"SELECT $1 AS "scope", COUNT(*) AS "n" FROM "source_files" AS "f" "#.to_owned()
      + r#"INNER JOIN "sources" AS "s" ON ("s"."id" = "f"."source_id" AND "s"."label" = $2) "#
      + r#"WHERE "f"."source_id" = $3 "#
      + r#"GROUP BY substr("f"."path", $4, $5) "#
      + r#"HAVING COUNT(*) > $6 "#
      + r#"ORDER BY "n" DESC LIMIT $7 OFFSET $8"#
  );
  assert_eq!(params.len(), 8);
  Ok(())
}
