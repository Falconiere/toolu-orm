//! `TableRef`, `AliasedColumn` and the expression-tree `JoinCondition`: the SQL
//! they render and the parameters they bind, per dialect.
//!
//! The executed counterparts live in `toolu-orm-query`'s
//! `{rusqlite,libsql,postgres}_joins_test` binaries.

use toolu_orm_core::alias::{QualifiedColumn, TableRef};
use toolu_orm_core::column::{Integer, Text};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::{Expr, JoinCondition};
use toolu_orm_core::query_column::{Column, CommonOps, NumericOps, TextOps};
use toolu_orm_core::value::Value;

const ID: Column<Text> = Column::new("code_symbols", "id");
const OWNER: Column<Text> = Column::new("code_symbols", "owner");
const CREATED_AT: Column<Integer> = Column::new("code_symbols", "created_at");

#[test]
fn table_ref_renders_alias_and_doubles_embedded_quotes() {
  assert_eq!(TableRef::new("code_symbols").to_sql(), r#""code_symbols""#);
  assert_eq!(TableRef::new("code_symbols").qualifier(), "code_symbols");
  assert_eq!(TableRef::new("code_symbols").alias(), None);

  let old = TableRef::aliased("code_symbols", "old");
  assert_eq!(old.to_sql(), r#""code_symbols" AS "old""#);
  assert_eq!(old.table(), "code_symbols");
  assert_eq!(old.alias(), Some("old"));
  assert_eq!(old.qualifier(), "old");

  // A name carrying the delimiter cannot close the identifier early.
  let hostile = TableRef::aliased(r#"me"m"#, r#"o"ld"#);
  assert_eq!(hostile.to_sql(), r#""me""m" AS "o""ld""#);
  assert_eq!(
    hostile.column(&ID).qualified(),
    r#""o""ld"."id""#,
    "columns are addressed through the escaped alias"
  );

  // An alias equal to the table name is legal and rendered verbatim.
  assert_eq!(
    TableRef::aliased("code_symbols", "code_symbols").to_sql(),
    r#""code_symbols" AS "code_symbols""#
  );
}

#[test]
fn aliased_column_qualifies_with_the_alias_and_falls_back_to_the_table() {
  let old = TableRef::aliased("code_symbols", "old");
  let plain = TableRef::new("code_symbols");

  assert_eq!(old.column(&ID).qualified(), r#""old"."id""#);
  assert_eq!(old.column(&ID).qualifier(), "old");
  assert_eq!(old.column(&ID).name(), "id");

  assert_eq!(plain.column(&ID).qualified(), ID.qualified());
  assert_eq!(plain.column(&ID).qualified(), r#""code_symbols"."id""#);

  // Clone and Debug work for a marker type that implements neither.
  let original = old.column(&ID);
  let cloned = original.clone();
  assert_eq!(cloned.qualified(), original.qualified());
  assert!(format!("{cloned:?}").contains("AliasedColumn"));

  // Object-safe: one slice can mix a plain column and an aliased one.
  let mixed: Vec<&dyn QualifiedColumn> = vec![&ID, &original];
  assert_eq!(
    mixed.iter().map(|c| c.qualified()).collect::<Vec<_>>(),
    vec![
      r#""code_symbols"."id""#.to_owned(),
      r#""old"."id""#.to_owned()
    ]
  );
}

#[test]
fn aliased_column_supports_the_same_predicate_and_ordering_ops() {
  let old = TableRef::aliased("code_symbols", "old");
  let owner = old.column(&OWNER);
  let created = old.column(&CREATED_AT);

  assert_eq!(owner.asc().to_sql(), r#""old"."owner" ASC"#);
  assert_eq!(created.desc().to_sql(), r#""old"."created_at" DESC"#);

  let cases: Vec<(Expr, &str, usize)> = vec![
    (owner.eq("o1"), r#""old"."owner" = ?3"#, 1),
    (owner.ne("o1"), r#""old"."owner" != ?3"#, 1),
    (owner.like("o%"), r#""old"."owner" LIKE ?3"#, 1),
    (owner.is_null(), r#""old"."owner" IS NULL"#, 0),
    (owner.is_not_null(), r#""old"."owner" IS NOT NULL"#, 0),
    (
      owner.in_list(&[Value::from("o1"), Value::from("o2")]),
      r#""old"."owner" IN (?3, ?4)"#,
      2,
    ),
    (
      owner.not_in(&[Value::from("o1")]),
      r#""old"."owner" NOT IN (?3)"#,
      1,
    ),
    (created.gt(5), r#""old"."created_at" > ?3"#, 1),
    (created.lt(5), r#""old"."created_at" < ?3"#, 1),
    (created.gte(5), r#""old"."created_at" >= ?3"#, 1),
    (created.lte(5), r#""old"."created_at" <= ?3"#, 1),
    (
      created.between(5, 9),
      r#""old"."created_at" BETWEEN ?3 AND ?4"#,
      2,
    ),
  ];
  for (expr, expected_sql, expected_params) in cases {
    let (sql, params) = expr.to_sql_fragment_for(3, Dialect::Sqlite);
    assert_eq!(sql, expected_sql);
    assert_eq!(params.len(), expected_params, "for {expected_sql}");
  }
}

#[test]
fn column_to_column_comparisons_render_every_operator_without_params() {
  let old = TableRef::aliased("code_symbols", "old");
  let newer = TableRef::aliased("code_symbols", "newer");
  let left = old.column(&CREATED_AT);
  let right = newer.column(&CREATED_AT);

  let cases: Vec<(JoinCondition, &str)> = vec![
    (left.equals(&right), "="),
    (left.not_equals(&right), "!="),
    (left.less_than(&right), "<"),
    (left.less_or_equal(&right), "<="),
    (left.greater_than(&right), ">"),
    (left.greater_or_equal(&right), ">="),
  ];
  for (condition, op) in cases {
    let (sql, params) = condition.to_sql_fragment_for(1, Dialect::Sqlite);
    assert_eq!(
      sql,
      format!(r#""old"."created_at" {op} "newer"."created_at""#)
    );
    assert!(params.is_empty(), "a column comparison binds nothing");
  }

  // Either side may be plain or aliased, in any mix.
  let (sql, _) = CREATED_AT
    .less_than(&newer.column(&CREATED_AT))
    .to_sql_fragment_for(1, Dialect::Sqlite);
  assert_eq!(sql, r#""code_symbols"."created_at" < "newer"."created_at""#);
  let (sql, _) = newer
    .column(&CREATED_AT)
    .greater_than(&CREATED_AT)
    .to_sql_fragment_for(1, Dialect::Sqlite);
  assert_eq!(sql, r#""newer"."created_at" > "code_symbols"."created_at""#);
}

#[test]
fn join_condition_and_or_number_params_from_the_given_start() {
  let old = TableRef::aliased("code_symbols", "old");
  let newer = TableRef::aliased("code_symbols", "newer");

  let condition = old
    .column(&OWNER)
    .equals(&newer.column(&OWNER))
    .and(
      old
        .column(&CREATED_AT)
        .less_than(&newer.column(&CREATED_AT)),
    )
    .and(old.column(&OWNER).eq("o1"))
    .or(newer.column(&CREATED_AT).gt(100));

  let (sqlite_sql, sqlite_params) = condition.to_sql_fragment_for(2, Dialect::Sqlite);
  assert_eq!(
    sqlite_sql,
    concat!(
      r#"((("old"."owner" = "newer"."owner" AND "old"."created_at" < "newer"."created_at")"#,
      r#" AND "old"."owner" = ?2) OR "newer"."created_at" > ?3)"#
    )
  );
  assert_eq!(
    sqlite_params,
    vec![Value::Text("o1".to_owned()), Value::Integer(100)],
    "values come back in placeholder order"
  );

  let (pg_sql, pg_params) = condition.to_sql_fragment_for(2, Dialect::Postgres);
  assert!(pg_sql.ends_with(r#"AND "old"."owner" = $2) OR "newer"."created_at" > $3)"#));
  assert_eq!(pg_params, sqlite_params);

  // An Expr converts into an ON predicate and back.
  let from_expr: JoinCondition = OWNER.eq("o1").into();
  let (sql, params) = from_expr.to_sql_fragment_for(1, Dialect::Sqlite);
  assert_eq!(sql, r#""code_symbols"."owner" = ?1"#);
  assert_eq!(params, vec![Value::Text("o1".to_owned())]);

  let back: Expr = OWNER.equals(&ID).into();
  let (sql, params) = back.to_sql_fragment_for(1, Dialect::Sqlite);
  assert_eq!(sql, r#""code_symbols"."owner" = "code_symbols"."id""#);
  assert!(params.is_empty());
}
