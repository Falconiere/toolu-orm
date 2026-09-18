//! A correlated subquery in projection position, returning one value per row.

use crate::db::{setup_db, OwnerCountRow};
use crate::queries::items_with_owner_counts;

type Outcome = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn a_scalar_subquery_counts_each_items_owner_rows() -> Outcome {
  let conn = setup_db("composition_scalar_1").await?;

  let rows: Vec<OwnerCountRow> = items_with_owner_counts().fetch_all(&conn).await?;

  assert_eq!(
    rows
      .iter()
      .map(|row| (row.id.as_str(), row.owner_rows))
      .collect::<Vec<(&str, i64)>>(),
    vec![("i1", 1), ("i2", 0), ("i3", 0)]
  );
  Ok(())
}

/// The projection is evaluated per row, so `fetch_one` decodes the same value
/// the listing reports for the first row.
#[tokio::test]
async fn the_same_projection_survives_a_bounded_first_row_fetch() -> Outcome {
  let conn = setup_db("composition_scalar_2").await?;

  let row: OwnerCountRow = items_with_owner_counts().fetch_one(&conn).await?;

  assert_eq!((row.id.as_str(), row.owner_rows), ("i1", 1));
  Ok(())
}
