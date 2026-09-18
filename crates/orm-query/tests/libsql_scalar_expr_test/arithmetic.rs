//! AC-4 and AC-5: an arithmetic self-update, and binds landing in SELECT, SET
//! and WHERE of one statement in the order they are written.

use toolu_orm_core::expr::{like_pattern_literal, Scalar};
use toolu_orm_core::query_column::{CommonOps, TextOps};
use toolu_orm_query::select::SelectBuilder;
use toolu_orm_query::update::UpdateBuilder;

use crate::db::{self, Memory};
use crate::seed::{ACCESS_COUNT, BODY, CREATED_AT, CUTOFF, ID};
use crate::support::{all_memories, effective_time, Labelled, TestResult, ESCAPE};

#[tokio::test]
async fn an_arithmetic_self_update_raises_only_the_filtered_row() -> TestResult {
  let conn = db::setup_db().await?;
  let bump = || async {
    UpdateBuilder::new("memories")
      .set_scalar(&ACCESS_COUNT, Scalar::col(&ACCESS_COUNT) + Scalar::bind(1))
      .filter(ID.eq("m1"))
      .execute(&conn)
      .await
  };

  assert_eq!(bump().await?, 1);
  let after_one: Vec<Memory> = all_memories().order_by(ID.asc()).fetch_all(&conn).await?;
  assert_eq!(
    after_one
      .iter()
      .map(|row| row.access_count)
      .collect::<Vec<_>>(),
    vec![1, 5, 12, 0]
  );

  bump().await?;
  let after_two: Vec<Memory> = all_memories().filter(ID.eq("m1")).fetch_all(&conn).await?;
  assert_eq!(after_two.first().map(|row| row.access_count), Some(2));
  Ok(())
}

#[tokio::test]
async fn binds_land_in_select_set_and_where_in_statement_order() -> TestResult {
  let conn = db::setup_db().await?;

  let updated = UpdateBuilder::new("memories")
    .set_scalar(&BODY, Scalar::col(&BODY).concat(Scalar::bind(" (seen)")))
    .set_scalar(&ACCESS_COUNT, Scalar::col(&ACCESS_COUNT) + Scalar::bind(10))
    .filter(Scalar::col(&CREATED_AT).gte(Scalar::bind(CUTOFF)))
    .execute(&conn)
    .await?;
  assert_eq!(updated, 1);

  let rows: Vec<Labelled> = SelectBuilder::new("memories")
    .columns_raw(&["id"])
    .column_scalar(
      Scalar::case_when(
        Scalar::col(&ACCESS_COUNT).gte(Scalar::bind(10)),
        Scalar::bind("bumped"),
      )
      .otherwise(Scalar::bind("untouched")),
      "heat",
    )
    .column_scalar(Scalar::col(&BODY), "tagged")
    .filter(BODY.like_escape(
      format!("%{}%", like_pattern_literal("100%", ESCAPE)),
      ESCAPE,
    ))
    .order_by(effective_time()?.desc())
    .fetch_all(&conn)
    .await?;

  assert_eq!(
    rows,
    vec![Labelled {
      id: "m1".to_owned(),
      heat: "bumped".to_owned(),
      tagged: "100% cotton (seen)".to_owned(),
    }]
  );
  Ok(())
}
