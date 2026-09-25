#[path = "fixtures/lance.rs"]
pub mod lance;
#[path = "fixtures/matrix_support.rs"]
pub mod support;

use std::time::{SystemTime, UNIX_EPOCH};

use toolu_orm_core::{
  dialect::Dialect,
  expr::{OrderBy, Scalar},
  query_column::{CommonOps, NumericOps},
  value::Value,
};
use toolu_orm_query::select::{Cte, SelectBuilder};

use crate::support::{
  Fixture, GROUP_ID, GROUP_LABEL, ITEM_GROUP, ITEM_ID, ITEM_NAME, ITEM_SCORE, TestResult,
};

#[test]
fn bound_filter_projection_order_and_page_return_expected_rows() -> TestResult {
  let fixture = Fixture::new()?;
  let (sql, params) = SelectBuilder::new("items")
    .columns_qualified(&[&ITEM_ID, &ITEM_NAME])
    .filter(ITEM_SCORE.gt(15_i64))
    .order_by(ITEM_ID.asc())
    .limit(2)
    .offset(1)
    .to_sql_for(Dialect::Lance);
  assert_eq!(
    sql,
    r#"SELECT "items"."id", "items"."name" FROM "items" WHERE "items"."score" > ?1 ORDER BY "items"."id" ASC LIMIT ?2 OFFSET ?3"#
  );
  assert_eq!(
    params,
    vec![Value::Integer(15), Value::Integer(2), Value::Integer(1)]
  );
  let rows = fixture.rows(&sql, &params, |row| {
    Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
  })?;
  assert_eq!(
    rows,
    vec![(3, "three".to_owned()), (4, "orphan".to_owned())]
  );
  Ok(())
}

#[test]
fn offset_without_limit_uses_lance_syntax() -> TestResult {
  let fixture = Fixture::new()?;
  let (sql, params) = SelectBuilder::new("items")
    .columns_qualified(&[&ITEM_ID])
    .order_by(ITEM_ID.asc())
    .offset(2)
    .to_sql_for(Dialect::Lance);
  assert_eq!(
    sql,
    r#"SELECT "items"."id" FROM "items" ORDER BY "items"."id" ASC OFFSET ?1"#
  );
  assert_eq!(params, vec![Value::Integer(2)]);
  assert_eq!(
    fixture.rows(&sql, &params, |row| row.get::<_, i64>(0))?,
    vec![3, 4]
  );
  Ok(())
}

#[test]
fn now_epoch_expression_matches_the_system_clock() -> TestResult {
  let fixture = Fixture::new()?;
  let (sql, params) = SelectBuilder::raw()
    .column_expr(Dialect::Lance.now_epoch(), "epoch")
    .to_sql_for(Dialect::Lance);
  assert_eq!(sql, r#"SELECT floor(epoch(now()))::bigint AS "epoch""#);
  assert!(params.is_empty());
  let before: i64 = SystemTime::now()
    .duration_since(UNIX_EPOCH)?
    .as_secs()
    .try_into()?;
  let epochs = fixture.rows(&sql, &params, |row| row.get::<_, i64>(0))?;
  let after: i64 = SystemTime::now()
    .duration_since(UNIX_EPOCH)?
    .as_secs()
    .try_into()?;
  assert_eq!(epochs.len(), 1);
  assert!((before..=after).contains(&epochs[0]));
  Ok(())
}

#[test]
fn inner_and_left_join_preserve_matches_and_unmatched_rows() -> TestResult {
  let fixture = Fixture::new()?;
  let inner = SelectBuilder::new("items")
    .columns_qualified(&[&ITEM_ID])
    .column_as(&GROUP_LABEL, "label")
    .join("groups", GROUP_ID.equals(&ITEM_GROUP))
    .filter(GROUP_LABEL.eq("alpha"))
    .order_by(ITEM_ID.asc())
    .to_sql_for(Dialect::Lance);
  assert_eq!(
    inner.0,
    r#"SELECT "items"."id", "groups"."label" AS "label" FROM "items" INNER JOIN "groups" ON "groups"."id" = "items"."group_id" WHERE "groups"."label" = ?1 ORDER BY "items"."id" ASC"#
  );
  assert_eq!(inner.1, vec![Value::Text("alpha".to_owned())]);
  let rows = fixture.rows(&inner.0, &inner.1, |row| {
    Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
  })?;
  assert_eq!(rows, vec![(1, "alpha".to_owned()), (2, "alpha".to_owned())]);

  let left = SelectBuilder::new("items")
    .columns_qualified(&[&ITEM_ID])
    .column_as(&GROUP_LABEL, "label")
    .left_join("groups", GROUP_ID.equals(&ITEM_GROUP))
    .filter(ITEM_ID.eq(4_i64))
    .to_sql_for(Dialect::Lance);
  assert_eq!(
    left.0,
    r#"SELECT "items"."id", "groups"."label" AS "label" FROM "items" LEFT JOIN "groups" ON "groups"."id" = "items"."group_id" WHERE "items"."id" = ?1"#
  );
  assert_eq!(left.1, vec![Value::Integer(4)]);
  let rows = fixture.rows(&left.0, &left.1, |row| {
    Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?))
  })?;
  assert_eq!(rows, vec![(4, None)]);
  Ok(())
}

#[test]
fn distinct_group_by_and_having_apply_bound_filters() -> TestResult {
  let fixture = Fixture::new()?;
  let distinct = SelectBuilder::new("items")
    .columns_qualified(&[&ITEM_GROUP])
    .distinct()
    .filter(ITEM_SCORE.gt(15_i64))
    .order_by(ITEM_GROUP.asc())
    .to_sql_for(Dialect::Lance);
  assert_eq!(
    distinct.0,
    r#"SELECT DISTINCT "items"."group_id" FROM "items" WHERE "items"."score" > ?1 ORDER BY "items"."group_id" ASC"#
  );
  assert_eq!(distinct.1, vec![Value::Integer(15)]);
  let groups = fixture.rows(&distinct.0, &distinct.1, |row| row.get::<_, i64>(0))?;
  assert_eq!(groups, vec![1, 2, 99]);

  let grouped = SelectBuilder::new("items")
    .columns_qualified(&[&ITEM_GROUP])
    .column_scalar(Scalar::count_star(), "n")
    .filter(ITEM_SCORE.gt(5_i64))
    .group_by(&ITEM_GROUP)
    .having(Scalar::count_star().gt(Scalar::bind(1_i64)))
    .to_sql_for(Dialect::Lance);
  assert_eq!(
    grouped.0,
    r#"SELECT "items"."group_id", COUNT(*) AS "n" FROM "items" WHERE "items"."score" > ?1 GROUP BY "items"."group_id" HAVING COUNT(*) > ?2"#
  );
  assert_eq!(grouped.1, vec![Value::Integer(5), Value::Integer(1)]);
  let groups = fixture.rows(&grouped.0, &grouped.1, |row| {
    Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
  })?;
  assert_eq!(groups, vec![(1, 2)]);
  Ok(())
}

#[test]
fn cte_subquery_and_union_keep_bind_order_across_selects() -> TestResult {
  let fixture = Fixture::new()?;
  let cte = SelectBuilder::new("hits")
    .columns_raw(&["id"])
    .with(Cte::new(
      "hits",
      SelectBuilder::new("items")
        .columns_qualified(&[&ITEM_ID])
        .filter(ITEM_SCORE.gt(15_i64)),
    ))
    .filter(Scalar::sql(r#""hits"."id""#).lt(Scalar::bind(4_i64)))
    .order_by(OrderBy::alias_asc("id"))
    .to_sql_for(Dialect::Lance);
  assert_eq!(
    cte.0,
    r#"WITH "hits" AS (SELECT "items"."id" FROM "items" WHERE "items"."score" > ?1) SELECT "id" FROM "hits" WHERE "hits"."id" < ?2 ORDER BY "id" ASC"#
  );
  assert_eq!(cte.1, vec![Value::Integer(15), Value::Integer(4)]);
  assert_eq!(
    fixture.rows(&cte.0, &cte.1, |row| row.get::<_, i64>(0))?,
    vec![2, 3]
  );

  let nested = SelectBuilder::new("items")
    .columns_qualified(&[&ITEM_ID])
    .filter(ITEM_SCORE.gt(20_i64))
    .filter(
      Scalar::col(&ITEM_ID).in_subquery(
        SelectBuilder::new("items")
          .columns_qualified(&[&ITEM_ID])
          .filter(ITEM_GROUP.eq(2_i64)),
      ),
    )
    .to_sql_for(Dialect::Lance);
  assert_eq!(
    nested.0,
    r#"SELECT "items"."id" FROM "items" WHERE "items"."score" > ?1 AND "items"."id" IN (SELECT "items"."id" FROM "items" WHERE "items"."group_id" = ?2)"#
  );
  assert_eq!(nested.1, vec![Value::Integer(20), Value::Integer(2)]);
  assert_eq!(
    fixture.rows(&nested.0, &nested.1, |row| row.get::<_, i64>(0))?,
    vec![3]
  );

  let union = SelectBuilder::new("items")
    .columns_qualified(&[&ITEM_ID])
    .filter(ITEM_GROUP.eq(1_i64))
    .union(
      SelectBuilder::new("items")
        .columns_qualified(&[&ITEM_ID])
        .filter(ITEM_SCORE.gt(15_i64)),
    )
    .order_by(OrderBy::alias_asc("id"))
    .to_sql_for(Dialect::Lance);
  assert_eq!(
    union.0,
    r#"SELECT "items"."id" FROM "items" WHERE "items"."group_id" = ?1 UNION SELECT "items"."id" FROM "items" WHERE "items"."score" > ?2 ORDER BY "id" ASC"#
  );
  assert_eq!(union.1, vec![Value::Integer(1), Value::Integer(15)]);
  assert_eq!(
    fixture.rows(&union.0, &union.1, |row| row.get::<_, i64>(0))?,
    vec![1, 2, 3, 4]
  );
  Ok(())
}
