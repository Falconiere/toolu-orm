//! The NULL-sensitivity the issue calls out: `NOT EXISTS` and `NOT IN` answer
//! the same question differently once a NULL is involved, on real rows.

use crate::db::{ids, setup_db, IdRow};
use crate::queries::{items_in_owner_ids, items_not_in_owner_ids, items_with_no_owner};

type Outcome = Result<(), Box<dyn std::error::Error>>;

/// `i1` has a live owner; `i2` points at a missing one; `i3`'s key is NULL, so
/// no owner row matches it and a correlated `NOT EXISTS` keeps it.
#[test]
fn not_exists_keeps_the_rows_whose_owner_is_missing_or_null() -> Outcome {
  let conn = setup_db()?;

  let rows: Vec<IdRow> = items_with_no_owner().fetch_all(&conn)?;

  assert_eq!(ids(&rows), vec!["i2", "i3"]);
  Ok(())
}

/// The trap: `owners` holds a NULL id, so `owner_id NOT IN (SELECT id FROM
/// owners)` is never true and the query returns **nothing**. Same question,
/// same data, different answer — which is why `NOT EXISTS` is the NULL-safe
/// form.
#[test]
fn not_in_over_a_set_containing_null_returns_no_rows_at_all() -> Outcome {
  let conn = setup_db()?;

  let rows: Vec<IdRow> = items_not_in_owner_ids(false).fetch_all(&conn)?;

  assert!(ids(&rows).is_empty(), "got: {:?}", ids(&rows));
  Ok(())
}

/// Excluding the NULL from the inner set recovers `i2` — but not `i3`, whose
/// own key is NULL. Two distinct NULL effects, neither of which `NOT EXISTS`
/// has.
#[test]
fn not_in_over_a_null_free_set_still_drops_the_row_with_a_null_key() -> Outcome {
  let conn = setup_db()?;

  let rows: Vec<IdRow> = items_not_in_owner_ids(true).fetch_all(&conn)?;

  assert_eq!(ids(&rows), vec!["i2"]);
  Ok(())
}

/// The positive test is unaffected by the NULL: only the matched key is in.
#[test]
fn in_subquery_returns_only_the_matched_key() -> Outcome {
  let conn = setup_db()?;

  let rows: Vec<IdRow> = items_in_owner_ids().fetch_all(&conn)?;

  assert_eq!(ids(&rows), vec!["i1"]);
  Ok(())
}

#[test]
fn the_counted_and_existence_forms_agree_with_the_rows() -> Outcome {
  let conn = setup_db()?;

  assert_eq!(items_with_no_owner().count(&conn)?, 2);
  assert!(items_with_no_owner().exists(&conn)?);
  assert_eq!(items_not_in_owner_ids(false).count(&conn)?, 0);
  assert!(!items_not_in_owner_ids(false).exists(&conn)?);
  Ok(())
}
