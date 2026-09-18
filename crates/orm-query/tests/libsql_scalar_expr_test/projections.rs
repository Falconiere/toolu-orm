//! AC-7: `CASE` and string concatenation as selected outputs.

use toolu_orm_core::expr::Scalar;
use toolu_orm_core::query_column::CommonOps;
use toolu_orm_query::select::SelectBuilder;

use crate::db;
use crate::seed::{ACCESS_COUNT, BODY, ID};
use crate::support::{Labelled, MaybeLabel, TestResult};

#[tokio::test]
async fn case_and_concatenation_return_their_branch_values() -> TestResult {
  let conn = db::setup_db().await?;
  let prefix = Scalar::func(
    "substr",
    vec![Scalar::col(&BODY), Scalar::bind(1), Scalar::bind(3)],
  )?;

  let rows: Vec<Labelled> = SelectBuilder::new("memories")
    .columns_raw(&["id"])
    .column_scalar(
      Scalar::case_when(
        Scalar::col(&ACCESS_COUNT).gt(Scalar::bind(4)),
        Scalar::bind("hot"),
      )
      .otherwise(Scalar::bind("cold")),
      "heat",
    )
    .column_scalar(
      Scalar::col(&ID).concat(Scalar::bind(":")).concat(prefix),
      "tagged",
    )
    .filter(ID.in_list(&["m1".into(), "m2".into()]))
    .order_by(ID.asc())
    .fetch_all(&conn)
    .await?;

  assert_eq!(
    rows,
    vec![
      Labelled {
        id: "m1".to_owned(),
        heat: "cold".to_owned(),
        tagged: "m1:100".to_owned(),
      },
      Labelled {
        id: "m2".to_owned(),
        heat: "hot".to_owned(),
        tagged: "m2:100".to_owned(),
      },
    ]
  );
  Ok(())
}

#[tokio::test]
async fn a_case_without_else_yields_null_when_no_branch_matches() -> TestResult {
  let conn = db::setup_db().await?;

  let rows: Vec<MaybeLabel> = SelectBuilder::new("memories")
    .column_scalar(
      Scalar::case_when(
        Scalar::col(&ACCESS_COUNT).gt(Scalar::bind(100)),
        Scalar::bind("busy"),
      )
      .end(),
      "label",
    )
    .filter(ID.eq("m1"))
    .fetch_all(&conn)
    .await?;

  assert_eq!(rows, vec![MaybeLabel { label: None }]);
  Ok(())
}
