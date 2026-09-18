//! AC-10 on Postgres: a grouping key from a joined, aliased relation.

use toolu_orm_core::expr::Scalar;
use toolu_orm_query::select::SelectBuilder;

use crate::db::{self, pairs, KeyCount};
use crate::seed::{LABEL, SOURCE_ID, SOURCE_PK};
use crate::support::{files, sources, TestResult};

#[tokio::test]
async fn grouping_by_a_joined_aliased_column_counts_per_label() -> TestResult {
  let client = db::setup_db("grouping_qualified_a").await?;
  let f = files();
  let s = sources();

  let rows: Vec<KeyCount> = SelectBuilder::from_table(&f)
    .column_as(&s.column(&LABEL), "label")
    .column_scalar(Scalar::count_star(), "n")
    .join(&s, s.column(&SOURCE_PK).equals(&f.column(&SOURCE_ID)))
    .group_by(&s.column(&LABEL))
    .order_by(s.column(&LABEL).asc())
    .fetch_all(&client)
    .await?;

  assert_eq!(pairs(&rows), vec![("primary", 6), ("secondary", 1)]);
  Ok(())
}
